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
        unsafe {
            // Create the channel
            let (sender, rx) = oneshot::channel::<errors::Result<Instance>>();
            let tx = Box::new(sender);
            // First argument: the address of the channel Sender
            let raw_ptr = Box::into_raw(tx);
            // Find the address of tx
            let address_string = format!("{:p}", raw_ptr);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();

            // Second argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&method_name, self.jni_env)?;

            // Rest of the arguments: Create a new objectarray of class InvocationArg
            let size = inv_args.len() as i32;
            let array_ptr = {
                let j = (opt_to_res(cache::get_jni_new_object_array())?)(
                    self.jni_env,
                    size,
                    cache::get_invocation_arg_class()?,
                    ptr::null_mut(),
                );
                jni_utils::create_global_ref_from_local_ref(j, self.jni_env)?
            };
            let mut inv_arg_jobjects: Vec<jobject> = Vec::with_capacity(size as usize);

            // Rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java =
                    inv_args[i as usize].as_java_ptr_with_global_ref(self.jni_env)?;
                // Set it in the array
                (opt_to_res(cache::get_jni_set_object_array_element())?)(
                    self.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
                inv_arg_jobjects.push(inv_arg_java);
            }

            // Call the method of the instance
            let _ = (opt_to_res(cache::get_jni_call_void_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_invoke_async_method()?,
                address,
                method_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(self.jni_env, ())?;

            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(self.jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(self.jni_env, array_ptr);
            jni_utils::delete_java_ref(self.jni_env, method_name_jstring);

            // Create and return the Instance
            let instance = rx.await?;
            Self::do_return(self.jni_env, instance)?
        }
    }
}

#[cfg(test)]
mod api_unit_tests {
    use super::*;
    use crate::JvmBuilder;
    use tokio;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invoke_async_success_w_tokio() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = JvmBuilder::new().build()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
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
        let jvm = JvmBuilder::new().build()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
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
        let jvm = JvmBuilder::new().build()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
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
        let jvm = JvmBuilder::new().build()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
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
        let jvm = JvmBuilder::new().build()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
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
        let jvm = JvmBuilder::new().build()?;
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
        let jvm = JvmBuilder::new().build()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
        let instance_result = jvm
            .invoke_async(&my_test, "echo", &[InvocationArg::try_from(s_test)?])
            .await;
        assert!(instance_result.is_err());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invoke_void_future() -> errors::Result<()> {
        let s_test = "j4rs_rust";
        let jvm = JvmBuilder::new().build()?;
        let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
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

    // #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn _memory_leaks_invoke_async_instances() -> errors::Result<()> {
        let jvm = JvmBuilder::new().build()?;
        let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
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
