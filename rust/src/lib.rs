// Copyright 2018 astonbitecode
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

#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate serde;
extern crate serde_json;

use futures::channel::oneshot;
use std::mem;
use std::os::raw::c_void;
use std::sync::mpsc::Sender;

pub use jni_sys;
use jni_sys::{jlong, jobject, jstring, JNIEnv};

pub use api::instance::Instance;
pub use api::instance::InstanceReceiver;

pub use self::api::invocation_arg::InvocationArg;
pub use self::api::Callback;
pub use self::api::ClasspathEntry;
pub use self::api::JavaClass;
pub use self::api::JavaOpt;
pub use self::api::Jvm;
pub use self::api::JvmBuilder;
pub use self::api::Null;
pub use self::api_tweaks::{get_created_java_vms, set_java_vm};
pub use self::jni_utils::jstring_to_rust_string;
pub use self::provisioning::LocalJarArtifact;
pub use self::provisioning::MavenArtifact;
pub use self::provisioning::MavenArtifactRepo;
pub use self::provisioning::MavenSettings;

mod api;
pub(crate) mod api_tweaks;
pub mod async_api;
mod cache;
pub mod errors;
pub mod jfx;
mod jni_utils;
mod logger;
pub mod prelude;
mod provisioning;
mod utils;

/// Creates a new JVM, using the provided classpath entries and JVM arguments
pub fn new_jvm(
    classpath_entries: Vec<ClasspathEntry>,
    java_opts: Vec<JavaOpt>,
) -> errors::Result<Jvm> {
    JvmBuilder::new()
        .classpath_entries(classpath_entries)
        .java_opts(java_opts)
        .build()
}

#[no_mangle]
pub extern "C" fn Java_org_astonbitecode_j4rs_api_invocation_NativeCallbackToRustChannelSupport_docallbacktochannel(
    _jni_env: *mut JNIEnv,
    _class: *const c_void,
    ptr_address: jlong,
    java_instance: jobject,
) {
    let mut jvm = Jvm::attach_thread()
        .expect("Could not create a j4rs Jvm while invoking callback to channel.");
    jvm.detach_thread_on_drop(false);
    let instance_res = Instance::from_jobject_with_global_ref(java_instance);
    if let Ok(instance) = instance_res {
        let p = ptr_address as *mut Sender<Instance>;
        let tx = unsafe { Box::from_raw(p) };

        let result = tx.send(instance);
        mem::forget(tx);
        if let Err(error) = result {
            panic!(
                "Could not send to the defined callback channel: {:?}",
                error
            );
        }
    } else {
        panic!("Could not create Rust Instance from the Java Instance object...");
    }
}

#[no_mangle]
pub extern "C" fn Java_org_astonbitecode_j4rs_api_invocation_NativeCallbackToRustFutureSupport_docallbacktochannel(
    _jni_env: *mut JNIEnv,
    _class: *const c_void,
    ptr_address: jlong,
    java_instance: jobject,
) {
    let mut jvm = Jvm::attach_thread().expect(
        "Could not create a j4rs Jvm while invoking callback to channel for completing a Future.",
    );
    jvm.detach_thread_on_drop(false);
    let instance_res = Instance::from_jobject_with_global_ref(java_instance);
    if let Ok(instance) = instance_res {
        let p = ptr_address as *mut oneshot::Sender<errors::Result<Instance>>;
        let tx = unsafe { Box::from_raw(p) };

        let result = tx.send(Ok(instance));
        if let Err(_) = result {
            panic!("Could not send to the defined callback channel to complete the future");
        }
    } else {
        panic!("Could not create Rust Instance from the Java Instance object...");
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_org_astonbitecode_j4rs_api_invocation_NativeCallbackToRustFutureSupport_failcallbacktochannel(
    _jni_env: *mut JNIEnv,
    _class: *const c_void,
    ptr_address: jlong,
    stacktrace: jstring,
) {
    let mut jvm = Jvm::attach_thread().expect(
        "Could not create a j4rs Jvm while invoking callback to channel for failing a Future.",
    );
    jvm.detach_thread_on_drop(false);
    let stacktrace = jstring_to_rust_string(&jvm, stacktrace);
    if let Ok(st) = stacktrace {
        let p = ptr_address as *mut oneshot::Sender<errors::Result<Instance>>;
        let tx = unsafe { Box::from_raw(p) };

        let result = tx.send(Err(errors::J4RsError::JavaError(st)));
        if let Err(_) = result {
            panic!("Could not send to the defined callback channel to fail a future");
        }
    } else {
        panic!("Could not create Rust String from the Java jstring while invoking callback to channel for failing a Future...");
    }
}

#[cfg(test)]
mod lib_unit_tests {
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::path::MAIN_SEPARATOR;
    use std::thread::JoinHandle;
    use std::{thread, time};

    use fs_extra::remove_items;

    use crate::api::{self, JavaClass};
    use crate::provisioning::JavaArtifact;
    use crate::{LocalJarArtifact, MavenArtifactRepo, MavenSettings, Null};

    use super::utils::jassets_path;
    use super::{errors, InvocationArg, Jvm, JvmBuilder, MavenArtifact};

    fn create_tests_jvm() -> errors::Result<Jvm> {
        let jvm: Jvm = JvmBuilder::new().build()?;
        jvm.deploy_artifact(&MavenArtifact::from(format!("io.github.astonbitecode:j4rs-testing:{}", api::j4rs_version()).as_str()))?;
        Ok(jvm)
    }

    #[test]
    fn create_instance_and_invoke() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let instantiation_args = vec![InvocationArg::try_from("arg from Rust")?];
        let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref());
        match instance {
            Ok(i) => {
                let invocation_args = vec![InvocationArg::try_from(" ")?];
                let invocation_result = jvm.invoke(&i, "split", &invocation_args);
                assert!(invocation_result.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        };

        let instantiation_args_2 = vec![InvocationArg::try_from("arg from Rust")?];
        let instance_2 = jvm.create_instance("java.lang.String", instantiation_args_2.as_ref());
        match instance_2 {
            Ok(i) => {
                let invocation_args = vec![InvocationArg::try_from(" ")?];
                let invocation_result = jvm.invoke(&i, "split", &invocation_args);
                assert!(invocation_result.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        };

        let static_invocation_result =
            jvm.invoke_static("java.lang.System", "currentTimeMillis", InvocationArg::empty());
        assert!(static_invocation_result.is_ok());

        Ok(())
    }

    #[test]
    fn init_callback_channel() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        match jvm.create_instance(
            "org.astonbitecode.j4rs.tests.MySecondTest",
            InvocationArg::empty(),
        ) {
            Ok(i) => {
                let instance_receiver_res = jvm.init_callback_channel(&i);
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res?;
                assert!(jvm.invoke(&i, "performCallback", InvocationArg::empty()).is_ok());
                let res_chan = instance_receiver.rx().recv();
                let i = res_chan?;
                let res_to_rust = jvm.to_rust(i);
                assert!(res_to_rust.is_ok());
                let _: String = res_to_rust?;
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        Ok(())
    }

    #[test]
    fn callback_to_channel() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        match jvm.create_instance(
            "org.astonbitecode.j4rs.tests.MySecondTest",
            InvocationArg::empty(),
        ) {
            Ok(i) => {
                let instance_receiver_res =
                    jvm.invoke_to_channel(&i, "performCallback", InvocationArg::empty());
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res?;
                let res_chan = instance_receiver.rx().recv();
                let i = res_chan?;
                let res_to_rust = jvm.to_rust(i);
                assert!(res_to_rust.is_ok());
                let _: String = res_to_rust?;
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        Ok(())
    }

    #[test]
    fn multiple_callbacks_to_channel() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        match jvm.create_instance(
            "org.astonbitecode.j4rs.tests.MySecondTest",
            InvocationArg::empty(),
        ) {
            Ok(i) => {
                let instance_receiver_res =
                    jvm.invoke_to_channel(&i, "performTenCallbacks", InvocationArg::empty());
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res?;
                for _i in 0..10 {
                    let thousand_millis = time::Duration::from_millis(1000);
                    let res_chan = instance_receiver.rx().recv_timeout(thousand_millis);
                    let i = res_chan.unwrap();
                    let res_to_rust = jvm.to_rust(i);
                    assert!(res_to_rust.is_ok());
                    let _: String = res_to_rust?;
                }
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        Ok(())
    }

    #[test]
    fn multiple_callbacks_to_channel_from_multiple_threads() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        match jvm.create_instance(
            "org.astonbitecode.j4rs.tests.MySecondTest",
            InvocationArg::empty(),
        ) {
            Ok(i) => {
                let instance_receiver_res =
                    jvm.invoke_to_channel(&i, "performCallbackFromTenThreads", InvocationArg::empty());
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res?;
                for _i in 0..10 {
                    let thousand_millis = time::Duration::from_millis(1000);
                    let res_chan = instance_receiver.rx().recv_timeout(thousand_millis);
                    let i = res_chan.unwrap();
                    let res_to_rust = jvm.to_rust(i);
                    assert!(res_to_rust.is_ok());
                    let _: String = res_to_rust?;
                }
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance:  {:?}", error);
            }
        }

        Ok(())
    }

    // #[test]
    // #[ignore]
    fn _memory_leaks_invoke_instances_to_channel() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        match jvm.create_instance(
            "org.astonbitecode.j4rs.tests.MySecondTest",
            InvocationArg::empty(),
        ) {
            Ok(instance) => {
                for i in 0..100000000 {
                    let instance_receiver = jvm
                        .invoke_to_channel(&instance, "performCallback", InvocationArg::empty())
                        ?;
                    let thousand_millis = time::Duration::from_millis(1000);
                    let res = instance_receiver.rx().recv_timeout(thousand_millis);
                    if i % 100000 == 0 {
                        println!("{}: {}", i, res.is_ok());
                    }
                }
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        let thousand_millis = time::Duration::from_millis(1000);
        thread::sleep(thousand_millis);

        Ok(())
    }

    #[test]
    fn clone_instance() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        // Create a MyTest instance
        let i_result =
            jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty());
        assert!(i_result.is_ok());
        let i_arg = i_result?;

        // Create two clones of the instance
        let i1 = jvm.clone_instance(&i_arg)?;
        let i2 = jvm.clone_instance(&i_arg)?;
        // Use the clones as arguments
        let invocation_res = jvm.create_instance(
            "org.astonbitecode.j4rs.tests.MyTest",
            &vec![InvocationArg::from(i1)],
        );
        assert!(invocation_res.is_ok());
        let invocation_res = jvm.create_instance(
            "org.astonbitecode.j4rs.tests.MyTest",
            &vec![InvocationArg::from(i2)],
        );
        assert!(invocation_res.is_ok());

        Ok(())
    }

    //    #[test]
    //    #[ignore]
    fn _memory_leaks_create_instances() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        for i in 0..100000000 {
            match jvm.create_instance(
                "org.astonbitecode.j4rs.tests.MySecondTest",
                InvocationArg::empty(),
            ) {
                Ok(instance) => {
                    if i % 100000 == 0 {
                        println!("{}: {}", i, instance.class_name());
                    }
                }
                Err(error) => {
                    panic!("ERROR when creating Instance: {:?}", error);
                }
            }
        }
        let thousand_millis = time::Duration::from_millis(1000);
        thread::sleep(thousand_millis);

        Ok(())
    }

    //        #[test]
    //    #[ignore]
    fn _memory_leaks_invoke_instances() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty()) {
            Ok(instance) => {
                let inv_arg = InvocationArg::try_from("tests")?;
                for i in 0..100000000 {
                    if i % 100000 == 0 {
                        println!("{}", i);
                    }
                    jvm.invoke(&instance, "getMyWithArgs", &[&inv_arg])?;
                }
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        let thousand_millis = time::Duration::from_millis(1000);
        thread::sleep(thousand_millis);

        Ok(())
    }

    // #[test]
    // #[ignore]
    fn _memory_leaks_invoke_instances_and_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty()) {
            Ok(instance) => {
                for i in 0..100000000 {
                    let ret_instance = jvm
                        .invoke(
                            &instance,
                            "echo",
                            &[InvocationArg::try_from(33333333_i32)?],
                        )?;
                    let v: i32 = jvm.to_rust(ret_instance)?;
                    if i % 100000 == 0 {
                        println!("{}: {}", i, v);
                    }
                }
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        let thousand_millis = time::Duration::from_millis(1000);
        thread::sleep(thousand_millis);

        Ok(())
    }

    //    #[test]
    //    #[ignore]
    fn _memory_leaks_invoke_instances_w_new_invarg() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let mut string_arg_rust = "".to_string();
        for _ in 0..100 {
            string_arg_rust = format!("{}{}", string_arg_rust, "astring")
        }
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty()) {
            Ok(instance) => {
                for i in 0..100000000 {
                    if i % 100000 == 0 {
                        println!("{}", i);
                    }
                    let _ia = InvocationArg::try_from(&string_arg_rust)?;
                    jvm.invoke(&instance, "getMyWithArgs", &[_ia])?;
                }
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        let thousand_millis = time::Duration::from_millis(1000);
        thread::sleep(thousand_millis);

        Ok(())
    }

    //    #[test]
    //    #[ignore]
    fn _memory_leaks_create_instances_in_different_threads() -> errors::Result<()> {
        for i in 0..100000000 {
            thread::spawn(move || {
                let jvm = create_tests_jvm().unwrap();
                match jvm.create_instance(
                    "org.astonbitecode.j4rs.tests.MySecondTest",
                    InvocationArg::empty(),
                ) {
                    Ok(_) => {
                        if i % 100000 == 0 {
                            println!("{}", i);
                        }
                    }
                    Err(error) => {
                        panic!("ERROR when creating Instance: {:?}", error);
                    }
                };
            });

            let millis = time::Duration::from_millis(10);
            thread::sleep(millis);
        }

        Ok(())
    }

    #[test]
    fn cast() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        let instantiation_args = vec![InvocationArg::try_from("Hi")?];
        let instance = jvm
            .create_instance("java.lang.String", instantiation_args.as_ref())?;
        jvm.cast(&instance, "java.lang.Object")?;

        Ok(())
    }

    #[test]
    fn invoke_vec() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty()) {
            Ok(i) => {
                // Test using InvocationArgs
                let invocation_args = vec![
                    InvocationArg::try_from("arg1"),
                    InvocationArg::try_from("arg2"),
                    InvocationArg::try_from("arg3"),
                    InvocationArg::try_from("arg33"),
                ];
                let list_instance = jvm.java_list("java.lang.String", invocation_args)?;
                let res = jvm.invoke(&i, "list", &[InvocationArg::from(list_instance)]);
                assert!(res.is_ok());
                // Test using instances
                let instance = jvm.create_instance(
                    "java.lang.String",
                    &[InvocationArg::try_from("astring")?],
                );
                let list_instance = jvm.java_list("java.lang.String", vec![instance])?;
                let res = jvm.invoke(&i, "list", &[InvocationArg::from(list_instance)]);
                assert!(res.is_ok());
                // Test other types
                let list_instance = jvm
                    .java_list(JavaClass::String, vec!["arg1", "arg2", "arg3", "arg33"])?;
                let res = jvm.invoke(&i, "list", &[InvocationArg::from(list_instance)]);
                assert!(res.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        Ok(())
    }

    #[test]
    fn invoke_map() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty()) {
            Ok(i) => {
                let map = HashMap::from([("Potatoes", 3), ("Tomatoes", 33), ("Carrotoes", 333)]);
                let map_instance = jvm
                    .java_map(JavaClass::String, JavaClass::Integer, map)?;
                let res = jvm.invoke(&i, "map", &[InvocationArg::from(map_instance)]);
                assert!(res.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        Ok(())
    }

    #[test]
    fn multithread() -> errors::Result<()> {
        let v: Vec<JoinHandle<String>> = (0..10)
            .map(|i: i8| {
                let v = thread::spawn(move || {
                    let jvm = create_tests_jvm().unwrap();
                    let instantiation_args =
                        vec![InvocationArg::try_from(format!("Thread{}", i)).unwrap()];
                    let instance = jvm
                        .create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
                    let string: String = jvm.to_rust(instance).unwrap();
                    string
                });
                v
            })
            .collect();

        for jh in v {
            let str = jh.join();
            println!("{}", str.unwrap());
        }

        Ok(())
    }

    #[test]
    fn use_a_java_instance_in_different_thread() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let instantiation_args = vec![InvocationArg::try_from("3")?];
        let instance = jvm
            .create_instance("java.lang.String", instantiation_args.as_ref())
            ?;

        let jh = thread::spawn(move || {
            let jvm = create_tests_jvm()?;
            let res = jvm.invoke(&instance, "isEmpty", InvocationArg::empty());
            res
        });

        let join_res = jh.join();
        assert!(join_res.is_ok());
        assert!(join_res.unwrap().is_ok());

        Ok(())
    }

    #[test]
    fn drop_and_attach_main_thread() -> errors::Result<()> {
        let tid = format!("{:?}", thread::current().id());
        {
            let jvm = create_tests_jvm()?;
            let instantiation_args = vec![InvocationArg::try_from(tid.clone())?];
            let instance = jvm
                .create_instance("java.lang.String", instantiation_args.as_ref())
                ?;
            let ref tid_from_java: String = jvm.to_rust(instance)?;
            assert!(&tid == tid_from_java);
        }
        {
            let jvm = create_tests_jvm()?;
            let instantiation_args = vec![InvocationArg::try_from(tid.clone())?];
            let instance = jvm
                .create_instance("java.lang.String", instantiation_args.as_ref())
                ?;
            let ref tid_from_java: String = jvm.to_rust(instance)?;
            assert!(&tid == tid_from_java);
        }

        Ok(())
    }

    #[test]
    fn drop_and_attach_other_thread() -> errors::Result<()> {
        let _: Jvm = super::new_jvm(Vec::new(), Vec::new())?;
        let jh = thread::spawn(move || {
            let tid = format!("{:?}", thread::current().id());
            {
                let jvm = create_tests_jvm().unwrap();
                let instantiation_args = vec![InvocationArg::try_from(tid.clone()).unwrap()];
                let instance = jvm
                    .create_instance("java.lang.String", instantiation_args.as_ref())
                    .unwrap();
                let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
                assert!(&tid == tid_from_java);
            }
            {
                let jvm = create_tests_jvm().unwrap();
                let instantiation_args = vec![InvocationArg::try_from(tid.clone()).unwrap()];
                let instance = jvm
                    .create_instance("java.lang.String", instantiation_args.as_ref())
                    .unwrap();
                let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
                assert!(&tid == tid_from_java);
            }
            true
        });

        assert!(jh.join().unwrap());

        Ok(())
    }

    #[test]
    fn deploy_maven_artifact() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        assert!(jvm
            .deploy_artifact(&MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1"))
            .is_ok());
        let to_remove = format!(
            "{}{}j4rs-0.5.1.jar",
            jassets_path().unwrap().to_str().unwrap(),
            MAIN_SEPARATOR
        );
        let _ = remove_items(&vec![to_remove]);

        assert!(jvm.deploy_artifact(&UnknownArtifact {}).is_err());

        Ok(())
    }

    #[test]
    fn deploy_maven_artifact_from_more_artifactories() -> errors::Result<()> {
        let jvm: Jvm = JvmBuilder::new()
            .with_maven_settings(MavenSettings::new(vec![
                MavenArtifactRepo::from("myrepo1::https://my.repo.io/artifacts"),
                MavenArtifactRepo::from("myrepo2::https://my.other.repo.io/artifacts"),
            ]))
            .build()?;
        assert!(jvm
            .deploy_artifact(&MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1"))
            .is_ok());
        let to_remove = format!(
            "{}{}j4rs-0.5.1.jar",
            jassets_path().unwrap().to_str().unwrap(),
            MAIN_SEPARATOR
        );
        let _ = remove_items(&vec![to_remove]);

        Ok(())
    }

    #[test]
    fn deploy_local_artifact() -> errors::Result<()> {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new())?;
        assert!(jvm
            .deploy_artifact(&LocalJarArtifact::from("./non_existing.jar"))
            .is_err());

        Ok(())
    }

    struct UnknownArtifact {}

    impl JavaArtifact for UnknownArtifact {}

    #[test]
    fn variadic_constructor() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        let s1 = InvocationArg::try_from("abc")?;
        let s2 = InvocationArg::try_from("def")?;
        let s3 = InvocationArg::try_from("ghi")?;

        let arr_instance = jvm
            .create_java_array("java.lang.String", &vec![s1, s2, s3])
            ?;

        let test_instance = jvm
            .create_instance(
                "org.astonbitecode.j4rs.tests.MyTest",
                &[InvocationArg::from(arr_instance)],
            )
            ?;

        let i = jvm.invoke(&test_instance, "getMyString", InvocationArg::empty())?;

        let s: String = jvm.to_rust(i)?;
        assert!(s == "abc, def, ghi");

        Ok(())
    }

    #[test]
    fn variadic_string_method() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;

        let s1 = InvocationArg::try_from("abc")?;
        let s2 = InvocationArg::try_from("def")?;
        let s3 = InvocationArg::try_from("ghi")?;

        let arr_instance = jvm
            .create_java_array("java.lang.String", &vec![s1, s2, s3])
            ?;

        let i = jvm
            .invoke(
                &test_instance,
                "getMyWithArgsList",
                &vec![InvocationArg::from(arr_instance)],
            )
            ?;

        let s: String = jvm.to_rust(i)?;
        assert!(s == "abcdefghi");

        Ok(())
    }

    #[test]
    fn variadic_int_method() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;

        let s1 = InvocationArg::try_from(1)?;
        let s2 = InvocationArg::try_from(2)?;
        let s3 = InvocationArg::try_from(3)?;

        let arr_instance = jvm
            .create_java_array("java.lang.Integer", &vec![s1, s2, s3])
            ?;

        let i = jvm
            .invoke(
                &test_instance,
                "addInts",
                &vec![InvocationArg::from(arr_instance)],
            )
            ?;

        let num: i32 = jvm.to_rust(i)?;
        assert!(num == 6);

        Ok(())
    }

    #[test]
    fn variadic_long_primitive_method() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let values: Vec<i64> = vec![1, 2, 3];
        let jargs: Vec<_> = values
            .into_iter()
            .map(|v| {
                InvocationArg::try_from(v)
                    .unwrap()
                    .into_primitive()
                    .unwrap()
            })
            .collect();

        let arr_instance = jvm.create_java_array("long", &jargs)?;

        let _ = jvm
            .invoke_static(
                "org.astonbitecode.j4rs.tests.MyTest",
                "useLongPrimitivesArray",
                &vec![InvocationArg::from(arr_instance)],
            )
            ?;

        Ok(())
    }

    #[test]
    fn instance_invocation_chain_and_collect() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let instance = jvm
            .create_instance(
                "org.astonbitecode.j4rs.tests.MyTest",
                &vec![InvocationArg::try_from("string")?],
            )
            ?;

        let i1 = jvm
            .chain(&instance)
            ?
            .invoke(
                "appendToMyString",
                &vec![InvocationArg::try_from("_is_appended")?],
            )
            ?
            .invoke("length", InvocationArg::empty())
            ?
            .collect();

        let product: isize = jvm.to_rust(i1)?;

        assert!(product == 18);

        Ok(())
    }

    #[test]
    fn instance_invocation_chain_and_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let instance = jvm
            .create_instance(
                "org.astonbitecode.j4rs.tests.MyTest",
                &vec![InvocationArg::try_from("string")?],
            )
            ?;

        let product: isize = jvm
            .into_chain(instance)
            .invoke(
                "appendToMyString",
                &vec![InvocationArg::try_from("_is_appended")?],
            )
            ?
            .invoke("length", InvocationArg::empty())
            ?
            .to_rust()
            ?;

        assert!(product == 18);

        Ok(())
    }

    #[test]
    fn static_invocation_chain_and_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        let static_invocation = jvm.static_class("java.lang.System")?;

        let _: isize = jvm
            .into_chain(static_invocation)
            .invoke("currentTimeMillis", InvocationArg::empty())
            ?
            .to_rust()
            ?;

        Ok(())
    }

    #[test]
    fn access_class_field_and_enum() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        let static_invocation = jvm.static_class("java.lang.System")?;
        let field_instance_res = jvm.field(&static_invocation, "out");
        assert!(field_instance_res.is_ok());

        let access_mode_enum = jvm.static_class("java.nio.file.AccessMode")?;
        let access_mode_write = jvm.field(&access_mode_enum, "WRITE");
        assert!(access_mode_write.is_ok());

        Ok(())
    }

    #[test]
    fn java_hello_world() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        let system = jvm.static_class("java.lang.System")?;
        let _ = jvm
            .into_chain(system)
            .field("out")
            ?
            .invoke(
                "println",
                &vec![InvocationArg::try_from("Hello World")?],
            )
            ?
            .collect();

        Ok(())
    }

    #[test]
    fn parent_interface_method() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;

        let size: isize = jvm
            .into_chain(instance)
            .invoke("getMap", InvocationArg::empty())
            ?
            .cast("java.util.Map")
            ?
            .invoke("size", InvocationArg::empty())
            ?
            .to_rust()
            ?;

        assert!(size == 2);

        Ok(())
    }

    #[test]
    fn invoke_generic_method() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        // Create the MyTest instance
        let instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;

        // Retrieve the annotated Map
        let dummy_map = jvm.invoke(&instance, "getMap", InvocationArg::empty())?;

        // Put a new Map entry
        let _ = jvm
            .invoke(
                &dummy_map,
                "put",
                &vec![
                    InvocationArg::try_from("three")?,
                    InvocationArg::try_from(3)?,
                ],
            )
            ?;

        // Get the size of the new map and assert
        let size: isize = jvm
            .into_chain(dummy_map)
            .invoke("size", InvocationArg::empty())
            ?
            .to_rust()
            ?;

        assert!(size == 3);

        Ok(())
    }

    #[test]
    fn invoke_method_with_primitive_args() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        // Test the primitives in constructors.
        // The constructor of Integer takes a primitive int as an argument.
        let ia = InvocationArg::try_from(1_i32)
            ?
            .into_primitive()
            ?;
        let res1 = jvm.create_instance("java.lang.Integer", &[ia]);
        assert!(res1.is_ok());

        // Test the primitives in invocations.
        let ia1 = InvocationArg::try_from(1_i32)?;
        let ia2 = InvocationArg::try_from(1_i32)?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;
        let res2 = jvm.invoke(
            &test_instance,
            "addInts",
            &[ia1.into_primitive()?, ia2.into_primitive()?],
        );
        assert!(res2.is_ok());

        Ok(())
    }

    #[test]
    fn to_tust_returns_list() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;
        let list_instance = jvm
            .invoke(
                &test_instance,
                "getNumbersUntil",
                &[InvocationArg::try_from(10_i32)?],
            )
            ?;
        let vec: Vec<i32> = jvm.to_rust(list_instance)?;
        assert!(vec.len() == 10);

        Ok(())
    }

    #[test]
    fn basic_types() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;

        // By values
        let arg = InvocationArg::try_from(33_i8)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i8 = jvm.to_rust(i)?;
        assert!(ret == 33_i8);

        let arg = InvocationArg::try_from(33_i16)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i16 = jvm.to_rust(i)?;
        assert!(ret == 33_i16);

        let arg = InvocationArg::try_from(33_i32)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i32 = jvm.to_rust(i)?;
        assert!(ret == 33_i32);

        let arg = InvocationArg::try_from(33_i64)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i64 = jvm.to_rust(i)?;
        assert!(ret == 33_i64);

        let arg = InvocationArg::try_from(33.33_f32)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: f32 = jvm.to_rust(i)?;
        assert!(ret == 33.33_f32);

        let arg = InvocationArg::try_from(33.33_f64)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: f64 = jvm.to_rust(i)?;
        assert!(ret == 33.33_f64);

        // By reference
        let arg = InvocationArg::try_from(&33_i8)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i8 = jvm.to_rust(i)?;
        assert!(ret == 33_i8);

        let arg = InvocationArg::try_from(&33_i16)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i16 = jvm.to_rust(i)?;
        assert!(ret == 33_i16);

        let arg = InvocationArg::try_from(&33_i32)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i32 = jvm.to_rust(i)?;
        assert!(ret == 33_i32);

        let arg = InvocationArg::try_from(&33_i64)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: i64 = jvm.to_rust(i)?;
        assert!(ret == 33_i64);

        let arg = InvocationArg::try_from(&33.33_f32)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: f32 = jvm.to_rust(i)?;
        assert!(ret == 33.33_f32);

        let arg = InvocationArg::try_from(&33.33_f64)?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: f64 = jvm.to_rust(i)?;
        assert!(ret == 33.33_f64);

        Ok(())
    }

    #[test]
    fn vecs_arrays() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;

        let arg = InvocationArg::try_from([33_i8, 34_i8].as_slice())?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: Vec<i8> = jvm.to_rust(i)?;
        assert!(ret == vec![33_i8, 34_i8]);

        let arg = InvocationArg::try_from([33_i16, 34_i16].as_slice())?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: Vec<i16> = jvm.to_rust(i)?;
        assert!(ret == vec![33_i16, 34_i16]);

        let arg = InvocationArg::try_from([33_i32, 34_i32].as_slice())?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: Vec<i32> = jvm.to_rust(i)?;
        assert!(ret == vec![33_i32, 34_i32]);

        let arg = InvocationArg::try_from([33_i64, 34_i64].as_slice())?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: Vec<i64> = jvm.to_rust(i)?;
        assert!(ret == vec![33_i64, 34_i64]);

        let arg = InvocationArg::try_from([33_f32, 34_f32].as_slice())?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: Vec<f32> = jvm.to_rust(i)?;
        assert!(ret == vec![33_f32, 34_f32]);

        let arg = InvocationArg::try_from([33_f64, 34_f64].as_slice())?;
        let i = jvm.invoke(&test_instance, "echo", &[arg])?;
        let ret: Vec<f64> = jvm.to_rust(i)?;
        assert!(ret == vec![33_f64, 34_f64]);

        Ok(())
    }

    #[test]
    fn null_handling() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;
        let null = jvm.invoke(&test_instance, "getNullInteger", InvocationArg::empty())?;
        let list_instance = jvm
            .invoke(
                &test_instance,
                "getNumbersUntil",
                &[InvocationArg::from(null)],
            )
            ?;
        let vec: Vec<i32> = jvm.to_rust(list_instance)?;
        assert!(vec.is_empty());

        Ok(())
    }

    #[test]
    fn null_creation() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;
        let null = InvocationArg::try_from(Null::Of("java.lang.Integer"))?;
        let list_instance = jvm
            .invoke(&test_instance, "getNumbersUntil", &[null])
            ?;
        let vec: Vec<i32> = jvm.to_rust(list_instance)?;
        assert!(vec.is_empty());

        let null = InvocationArg::try_from(Null::Integer)?;
        let list_instance = jvm
            .invoke(&test_instance, "getNumbersUntil", &[null])
            ?;
        let vec: Vec<i32> = jvm.to_rust(list_instance)?;
        assert!(vec.is_empty());

        Ok(())
    }

    #[test]
    fn to_rust_boxed() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;

        let i = jvm
            .invoke(
                &test_instance,
                "echo",
                &vec![InvocationArg::try_from(true)?],
            )
            ?;
        let _: Box<bool> = jvm.to_rust_boxed(i)?;
        let i = jvm
            .invoke(
                &test_instance,
                "echo",
                &vec![InvocationArg::try_from(33_i8)?],
            )
            ?;
        let _: Box<i8> = jvm.to_rust_boxed(i)?;
        let i = jvm
            .invoke(
                &test_instance,
                "echo",
                &vec![InvocationArg::try_from(33_i16)?],
            )
            ?;
        let _: Box<i16> = jvm.to_rust_boxed(i)?;
        let i = jvm
            .invoke(
                &test_instance,
                "echo",
                &vec![InvocationArg::try_from(33_i32)?],
            )
            ?;
        let _: Box<i32> = jvm.to_rust_boxed(i)?;
        let i = jvm
            .invoke(
                &test_instance,
                "echo",
                &vec![InvocationArg::try_from(33_i64)?],
            )
            ?;
        let _: Box<i64> = jvm.to_rust_boxed(i)?;

        Ok(())
    }
}
