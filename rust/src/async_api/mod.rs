// Copyright 2023 astonbitecode
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::ptr;

use jni_sys::{jobject, jstring};

use futures::channel::oneshot;

use crate::errors::opt_to_res;
use crate::{cache, errors, jni_utils, Instance, InvocationArg, Jvm};

use super::logger::debug;

impl Jvm {
    /// Invokes the method `method_name` of a created `Instance` asynchronously, passing an array of `InvocationArg`s.
    /// It returns an `Instance` as the result of the invocation.
    pub async fn invoke_async(
        &self,
        instance: &Instance,
        method_name: &str,
        inv_args: &[InvocationArg],
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Asynchronously invoking method {} of class {} using {} arguments",
            method_name,
            instance.class_name,
            inv_args.len()
        ));
        // Create the channel
        let (sender, rx) = oneshot::channel::<errors::Result<Instance>>();
        unsafe {
            Self::handle_channel_sender(self, sender, &instance, &method_name, inv_args.as_ref())?;
        }
        // Create and return the Instance
        let instance = rx.await?;
        Self::do_return(self.jni_env, instance)?
    }

    /// Invokes the method `method_name` of a created `Instance` asynchronously, passing an array of `InvocationArg`s.
    /// It returns an `Instance` as the result of the invocation.
    /// 
    /// 
    /// `Instance`s  are `Send` and can be safely sent to other threads. However, because of [Send Approximation](https://rust-lang.github.io/async-book/07_workarounds/03_send_approximation.html), the `Future` returned by `invoke_async` is _not_ `Send`, even if it just contains an `Instance`. This is because the `Jvm` is being captured by the `async` call as well and the `Jvm` is __not__ `Send`.
    /// 
    /// In order to have a `Future<Instance>` that __is__ `Send`, the `Jvm::invoke_into_sendable_async` can be used. This function does not get a `Jvm` as argument; it creates one internally when needed and applies some scoping workarounds in order to achieve returning a `Future<Instance>` which is also `Send`.
    pub async fn invoke_into_sendable_async(
        instance: Instance,
        method_name: String,
        inv_args: Vec<InvocationArg>,
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Asynchronously invoking (2) method {} of class {} using {} arguments",
            method_name,
            instance.class_name,
            inv_args.len()
        ));
        // Create the channel
        let (sender, rx) = oneshot::channel::<errors::Result<Instance>>();
        unsafe {
            let s = Jvm::attach_thread()?;
            Self::handle_channel_sender(&s, sender, &instance, &method_name, inv_args.as_ref())?;
            drop(s);
        }

        // Create and return the Instance
        let instance = rx.await?;
        let new_jni_env = Jvm::attach_thread()?.jni_env;
        Self::do_return(new_jni_env, instance)?
    }

    unsafe fn handle_channel_sender(s: &Jvm, sender: oneshot::Sender<errors::Result<Instance>>, instance: &Instance, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<()> {
            let tx = Box::new(sender);
            // First argument: the address of the channel Sender
            let raw_ptr = Box::into_raw(tx);
            // Find the address of tx
            let address_string = format!("{:p}", raw_ptr);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();

            // Second argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&method_name, s.jni_env)?;

            // Rest of the arguments: Create a new objectarray of class InvocationArg
            let size = inv_args.len() as i32;
            let array_ptr = {
                let j = (opt_to_res(cache::get_jni_new_object_array())?)(
                    s.jni_env,
                    size,
                    cache::get_invocation_arg_class()?,
                    ptr::null_mut(),
                );
                jni_utils::create_global_ref_from_local_ref(j, s.jni_env)?
            };
            let mut inv_arg_jobjects: Vec<jobject> = Vec::with_capacity(size as usize);

            // Rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java =
                    inv_args[i as usize].as_java_ptr_with_global_ref(s.jni_env)?;
                // Set it in the array
                (opt_to_res(cache::get_jni_set_object_array_element())?)(
                    s.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
                inv_arg_jobjects.push(inv_arg_java);
            }

            // Call the method of the instance
            let _ = (opt_to_res(cache::get_jni_call_void_method())?)(
                s.jni_env,
                instance.jinstance,
                cache::get_invoke_async_method()?,
                address,
                method_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(s.jni_env, ())?;

            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(s.jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(s.jni_env, array_ptr);
            jni_utils::delete_java_ref(s.jni_env, method_name_jstring);
            Ok(())
    }
}

#[cfg(test)]
mod api_unit_tests {
    use super::*;
    use crate::{api, JvmBuilder, MavenArtifact};
    use futures::Future;
    use tokio;

    fn create_tests_jvm() -> errors::Result<Jvm> {
        let jvm: Jvm = JvmBuilder::new().build()?;
        jvm.deploy_artifact(&MavenArtifact::from(format!("io.github.astonbitecode:j4rs-testing:{}", api::j4rs_version()).as_str()))?;
        Ok(jvm)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invoke_async_success_w_tokio() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance = jvm
            .invoke_async(
                &my_test,
                "getStringWithFuture",
                &[InvocationArg::try_from(s_test)?],
            )
            .await?;
        let string: String = jvm.to_rust(instance)?;
        assert_eq!(s_test, string);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invoke_async_failure_w_tokio() -> errors::Result<()> {
        let s_test = "Boom!";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance_result = jvm
            .invoke_async(
                &my_test,
                "getErrorWithFuture",
                &[InvocationArg::try_from(s_test)?],
            )
            .await;
        assert!(instance_result.is_err());
        let error = instance_result.err().unwrap();
        println!("{}", error);
        Ok(())
    }

    #[async_std::test]
    async fn invoke_async_success_w_async_std() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance = jvm
            .invoke_async(
                &my_test,
                "getStringWithFuture",
                &[InvocationArg::try_from(s_test)?],
            )
            .await?;
        let string: String = jvm.to_rust(instance)?;
        assert_eq!(s_test, string);
        Ok(())
    }

    #[async_std::test]
    async fn invoke_async_failure_w_async_std() -> errors::Result<()> {
        let s_test = "Boom!";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance_result = jvm
            .invoke_async(
                &my_test,
                "getErrorWithFuture",
                &[InvocationArg::try_from(s_test)?],
            )
            .await;
        assert!(instance_result.is_err());
        let error = instance_result.err().unwrap();
        println!("{}", error);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invoke_async_and_reuse_instance() -> errors::Result<()> {
        let s_test1 = "j4rs_rust1";
        let s_test2 = "j4rs_rust2";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance1 = jvm
            .invoke_async(
                &my_test,
                "getStringWithFuture",
                &[InvocationArg::try_from(s_test1)?],
            )
            .await?;
        let instance2 = jvm
            .invoke_async(
                &my_test,
                "getStringWithFuture",
                &[InvocationArg::try_from(s_test2)?],
            )
            .await?;
        let string1: String = jvm.to_rust(instance1)?;
        let string2: String = jvm.to_rust(instance2)?;
        assert_eq!(s_test1, string1);
        assert_eq!(s_test2, string2);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn invoke_static_async() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.static_class("org.astonbitecode.j4rs.tests.MyTest")?;
        let instance = jvm
            .invoke_async(
                &my_test,
                "getErrorWithFutureStatic",
                &[InvocationArg::try_from(s_test)?],
            )
            .await?;
        let string: String = jvm.to_rust(instance)?;
        assert_eq!(s_test, string);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn invoke_async_error_before_executing_async() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance_result = jvm
            .invoke_async(&my_test, "echo", &[InvocationArg::try_from(s_test)?])
            .await;
        assert!(instance_result.is_err());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invoke_void_future() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance_res = jvm
            .invoke_async(
                &my_test,
                "executeVoidFuture",
                &[InvocationArg::try_from(s_test)?],
            )
            .await;
        assert!(instance_res.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invoke_into_sendable_async_success() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let instance = Jvm::invoke_into_sendable_async(
                my_test,
                "getStringWithFuture".to_string(),
                vec![InvocationArg::try_from(s_test)?],
            )
            .await?;
        let string: String = jvm.to_rust(instance)?;
        assert_eq!(s_test, string);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn future_is_send() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = create_tests_jvm()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())?;
        let f = Jvm::invoke_into_sendable_async(
            my_test,
            "executeVoidFuture".to_string(),
            vec![InvocationArg::try_from(s_test)?],
        );
        check_send(f);
        Ok(())
    }

    fn check_send<F:Future>(_:F) where F:Send + 'static {}

    // #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn _memory_leaks_invoke_async_instances() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty() as &[InvocationArg; 0])?;
        for i in 0..100000000 {
            if i % 100000 == 0 {
                println!("{}", i);
            }
            let ia = InvocationArg::try_from(i.to_string())?;
            let _s = jvm
                .invoke_async(&instance, "getStringWithFuture", &[ia])
                .await?;
        }
        Ok(())
    }
}
