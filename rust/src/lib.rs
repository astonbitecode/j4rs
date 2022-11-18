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

use std::mem;
use std::os::raw::c_void;
use std::sync::mpsc::Sender;

use jni_sys::{jlong, JNIEnv, jobject};
pub use jni_sys as jni_sys;

pub use self::api::Callback as Callback;
pub use self::api::ClasspathEntry as ClasspathEntry;
pub use api::instance::Instance as Instance;
pub use api::instance::InstanceReceiver as InstanceReceiver;
pub use self::api::invocation_arg::InvocationArg as InvocationArg;
pub use self::api::JavaOpt as JavaOpt;
pub use self::api::Jvm as Jvm;
pub use self::api::JvmBuilder as JvmBuilder;
pub use self::api::Null as Null;
pub use self::api::JavaClass as JavaClass;
pub use self::api_tweaks::{get_created_java_vms, set_java_vm};
pub use self::jni_utils::jstring_to_rust_string as jstring_to_rust_string;
pub use self::provisioning::LocalJarArtifact as LocalJarArtifact;
pub use self::provisioning::MavenArtifact as MavenArtifact;
pub use self::provisioning::MavenArtifactRepo as MavenArtifactRepo;
pub use self::provisioning::MavenSettings as MavenSettings;

mod api;
pub(crate) mod api_tweaks;
pub mod errors;
mod jni_utils;
mod logger;
mod provisioning;
mod utils;
mod cache;
pub mod prelude;
pub mod jfx;

/// Creates a new JVM, using the provided classpath entries and JVM arguments
pub fn new_jvm(classpath_entries: Vec<ClasspathEntry>, java_opts: Vec<JavaOpt>) -> errors::Result<Jvm> {
    JvmBuilder::new()
        .classpath_entries(classpath_entries)
        .java_opts(java_opts)
        .build()
}

#[no_mangle]
pub extern fn Java_org_astonbitecode_j4rs_api_invocation_NativeCallbackToRustChannelSupport_docallbacktochannel(_jni_env: *mut JNIEnv, _class: *const c_void, ptr_address: jlong, java_instance: jobject) {
    let mut jvm = Jvm::attach_thread().expect("Could not create a j4rs Jvm while invoking callback to channel.");
    jvm.detach_thread_on_drop(false);
    let instance_res = Instance::from_jobject_with_global_ref(java_instance);
    if let Ok(instance) = instance_res {
        let p = ptr_address as *mut Sender<Instance>;
        let tx = unsafe { Box::from_raw(p) };

        let result = tx.send(instance);
        mem::forget(tx);
        if let Err(error) = result {
            panic!("Could not send to the defined callback channel: {:?}", error);
        }
    } else {
        panic!("Could not create Rust Instance from the Java Instance object...");
    }
}

#[cfg(test)]
mod lib_unit_tests {
    use std::{thread, time};
    use std::convert::TryFrom;
    use std::path::MAIN_SEPARATOR;
    use std::thread::JoinHandle;

    use fs_extra::remove_items;

    use crate::{LocalJarArtifact, MavenArtifactRepo, MavenSettings, Null};
    use crate::api::JavaClass;
    use crate::provisioning::JavaArtifact;

    use super::{ClasspathEntry, InvocationArg, Jvm, JvmBuilder, MavenArtifact};
    use super::utils::jassets_path;

    #[test]
    fn create_instance_and_invoke() {
        let jvm: Jvm = JvmBuilder::new()
            .classpath_entry(ClasspathEntry::new("onemore.jar"))
            .build()
            .unwrap();

        let instantiation_args = vec![InvocationArg::try_from("arg from Rust").unwrap()];
        let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref());
        match instance {
            Ok(i) => {
                let invocation_args = vec![InvocationArg::try_from(" ").unwrap()];
                let invocation_result = jvm.invoke(&i, "split", &invocation_args);
                assert!(invocation_result.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        };

        let instantiation_args_2 = vec![InvocationArg::try_from("arg from Rust").unwrap()];
        let instance_2 = jvm.create_instance("java.lang.String", instantiation_args_2.as_ref());
        match instance_2 {
            Ok(i) => {
                let invocation_args = vec![InvocationArg::try_from(" ").unwrap()];
                let invocation_result = jvm.invoke(&i, "split", &invocation_args);
                assert!(invocation_result.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        };

        let static_invocation_result = jvm.invoke_static("java.lang.System", "currentTimeMillis", &Vec::new());
        assert!(static_invocation_result.is_ok());
    }

    #[test]
    fn init_callback_channel() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
            Ok(i) => {
                let instance_receiver_res = jvm.init_callback_channel(&i);
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res.unwrap();
                assert!(jvm.invoke(&i, "performCallback", &vec![]).is_ok());
                let res_chan = instance_receiver.rx().recv();
                let i = res_chan.unwrap();
                let res_to_rust = jvm.to_rust(i);
                assert!(res_to_rust.is_ok());
                let _: String = res_to_rust.unwrap();
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }
    }

    #[test]
    fn callback_to_channel() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
            Ok(i) => {
                let instance_receiver_res = jvm.invoke_to_channel(&i, "performCallback", Vec::new().as_ref());
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res.unwrap();
                let res_chan = instance_receiver.rx().recv();
                let i = res_chan.unwrap();
                let res_to_rust = jvm.to_rust(i);
                assert!(res_to_rust.is_ok());
                let _: String = res_to_rust.unwrap();
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }
    }

    #[test]
    fn multiple_callbacks_to_channel() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
            Ok(i) => {
                let instance_receiver_res = jvm.invoke_to_channel(&i, "performTenCallbacks", Vec::new().as_ref());
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res.unwrap();
                for _i in 0..10 {
                    let thousand_millis = time::Duration::from_millis(1000);
                    let res_chan = instance_receiver.rx().recv_timeout(thousand_millis);
                    let i = res_chan.unwrap();
                    let res_to_rust = jvm.to_rust(i);
                    assert!(res_to_rust.is_ok());
                    let _: String = res_to_rust.unwrap();
                }
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }
    }

    #[test]
    fn multiple_callbacks_to_channel_from_multiple_threads() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
            Ok(i) => {
                let instance_receiver_res = jvm.invoke_to_channel(&i, "performCallbackFromTenThreads", Vec::new().as_ref());
                assert!(instance_receiver_res.is_ok());
                let instance_receiver = instance_receiver_res.unwrap();
                for _i in 0..10 {
                    let thousand_millis = time::Duration::from_millis(1000);
                    let res_chan = instance_receiver.rx().recv_timeout(thousand_millis);
                    let i = res_chan.unwrap();
                    let res_to_rust = jvm.to_rust(i);
                    assert!(res_to_rust.is_ok());
                    let _: String = res_to_rust.unwrap();
                }
                let millis = time::Duration::from_millis(500);
                thread::sleep(millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance:  {:?}", error);
            }
        }
    }

    // #[test]
    // #[ignore]
    fn _memory_leaks_invoke_instances_to_channel() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
            Ok(instance) => {
                for i in 0..100000000 {
                    let instance_receiver = jvm.invoke_to_channel(&instance, "performCallback", &[]).unwrap();
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
    }

    #[test]
    fn clone_instance() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();
        // Create a MyTest instance
        let i_result = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref());
        assert!(i_result.is_ok());
        let i_arg = i_result.unwrap();

        // Create two clones of the instance
        let i1 = jvm.clone_instance(&i_arg).unwrap();
        let i2 = jvm.clone_instance(&i_arg).unwrap();
        // Use the clones as arguments
        let invocation_res = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &vec![InvocationArg::from(i1)]);
        assert!(invocation_res.is_ok());
        let invocation_res = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &vec![InvocationArg::from(i2)]);
        assert!(invocation_res.is_ok());
    }

    //    #[test]
//    #[ignore]
    fn _memory_leaks_create_instances() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();

        for i in 0..100000000 {
            match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
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
    }

    //        #[test]
//    #[ignore]
    fn _memory_leaks_invoke_instances() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
            Ok(instance) => {
                for i in 0..100000000 {
                    if i % 100000 == 0 {
                        println!("{}", i);
                    }
                    jvm.invoke(&instance, "getMyString", &[]).unwrap();
                }
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        let thousand_millis = time::Duration::from_millis(1000);
        thread::sleep(thousand_millis);
    }

    // #[test]
    // #[ignore]
    fn _memory_leaks_invoke_instances_and_to_rust() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
            Ok(instance) => {
                for i in 0..100000000 {
                    let ret_instance = jvm.invoke(&instance, "echo", &[InvocationArg::try_from(33333333_i32).unwrap()]).unwrap();
                    let v: i32 = jvm.to_rust(ret_instance).unwrap();
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
    }

    //    #[test]
//    #[ignore]
    fn _memory_leaks_invoke_instances_w_new_invarg() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let mut string_arg_rust = "".to_string();
        for _ in 0..100 {
            string_arg_rust = format!("{}{}", string_arg_rust, "astring")
        }
        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
            Ok(instance) => {
                for i in 0..100000000 {
                    if i % 100000 == 0 {
                        println!("{}", i);
                    }
                    let _ia = InvocationArg::try_from(&string_arg_rust).unwrap();
                    jvm.invoke(&instance, "getMyWithArgs", &[_ia]).unwrap();
                }
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }

        let thousand_millis = time::Duration::from_millis(1000);
        thread::sleep(thousand_millis);
    }

    //    #[test]
//    #[ignore]
    fn _memory_leaks_create_instances_in_different_threads() {
        for i in 0..100000000 {
            thread::spawn(move || {
                let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
                match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
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
    }

    #[test]
    fn cast() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], vec![]).unwrap();

        let instantiation_args = vec![InvocationArg::try_from("Hi").unwrap()];
        let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
        jvm.cast(&instance, "java.lang.Object").unwrap();
    }

    #[test]
    fn invoke_vec() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], vec![]).unwrap();

        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
            Ok(i) => {
                let list_instance = jvm.java_list(
                    JavaClass::String,
                    vec!["arg1", "arg2", "arg3", "arg33"])
                    .unwrap();
                let res = jvm.invoke(&i, "list", &[InvocationArg::from(list_instance)]);
                assert!(res.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }
    }

    #[test]
    fn multithread() {
        let v: Vec<JoinHandle<String>> = (0..10)
            .map(|i: i8| {
                let v = thread::spawn(move || {
                    let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
                    let instantiation_args = vec![InvocationArg::try_from(format!("Thread{}", i)).unwrap()];
                    let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
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
    }

    #[test]
    fn use_a_java_instance_in_different_thread() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let instantiation_args = vec![InvocationArg::try_from("3").unwrap()];
        let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();

        let jh = thread::spawn(move || {
            let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
            let res = jvm.invoke(&instance, "isEmpty", &Vec::new());
            res
        });

        let join_res = jh.join();
        assert!(join_res.is_ok());
        assert!(join_res.unwrap().is_ok());
    }

    #[test]
    fn drop_and_attach_main_thread() {
        let tid = format!("{:?}", thread::current().id());
        {
            let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
            let instantiation_args = vec![InvocationArg::try_from(tid.clone()).unwrap()];
            let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
            let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
            assert!(&tid == tid_from_java);
        }
        {
            let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
            let instantiation_args = vec![InvocationArg::try_from(tid.clone()).unwrap()];
            let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
            let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
            assert!(&tid == tid_from_java);
        }
    }

    #[test]
    fn drop_and_attach_other_thread() {
        let _: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let jh = thread::spawn(move || {
            let tid = format!("{:?}", thread::current().id());
            {
                let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
                let instantiation_args = vec![InvocationArg::try_from(tid.clone()).unwrap()];
                let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
                let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
                assert!(&tid == tid_from_java);
            }
            {
                let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
                let instantiation_args = vec![InvocationArg::try_from(tid.clone()).unwrap()];
                let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
                let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
                assert!(&tid == tid_from_java);
            }
            true
        });

        assert!(jh.join().unwrap());
    }

    #[test]
    fn deploy_maven_artifact() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        assert!(jvm.deploy_artifact(&MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1")).is_ok());
        let to_remove = format!("{}{}j4rs-0.5.1.jar", jassets_path().unwrap().to_str().unwrap(), MAIN_SEPARATOR);
        let _ = remove_items(&vec![to_remove]);

        assert!(jvm.deploy_artifact(&UnknownArtifact {}).is_err());
    }

    #[test]
    fn deploy_maven_artifact_from_more_artifactories() {
        let jvm: Jvm = JvmBuilder::new()
            .with_maven_settings(MavenSettings::new(vec![
                MavenArtifactRepo::from("myrepo1::https://my.repo.io/artifacts"),
                MavenArtifactRepo::from("myrepo2::https://my.other.repo.io/artifacts")])
            )
            .build()
            .unwrap();
        assert!(jvm.deploy_artifact(&MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1")).is_ok());
        let to_remove = format!("{}{}j4rs-0.5.1.jar", jassets_path().unwrap().to_str().unwrap(), MAIN_SEPARATOR);
        let _ = remove_items(&vec![to_remove]);
    }

    #[test]
    fn deploy_local_artifact() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        assert!(jvm.deploy_artifact(&LocalJarArtifact::from("./non_existing.jar")).is_err());
    }

    struct UnknownArtifact {}

    impl JavaArtifact for UnknownArtifact {}

    #[test]
    fn variadic_constructor() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();

        let s1 = InvocationArg::try_from("abc").unwrap();
        let s2 = InvocationArg::try_from("def").unwrap();
        let s3 = InvocationArg::try_from("ghi").unwrap();

        let arr_instance = jvm.create_java_array("java.lang.String", &vec![s1, s2, s3]).unwrap();

        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[InvocationArg::from(arr_instance)]).unwrap();

        let i = jvm.invoke(&test_instance, "getMyString", &[]).unwrap();

        let s: String = jvm.to_rust(i).unwrap();
        assert!(s == "abc, def, ghi");
    }

    #[test]
    fn variadic_string_method() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();

        let s1 = InvocationArg::try_from("abc").unwrap();
        let s2 = InvocationArg::try_from("def").unwrap();
        let s3 = InvocationArg::try_from("ghi").unwrap();

        let arr_instance = jvm.create_java_array("java.lang.String", &vec![s1, s2, s3]).unwrap();

        let i = jvm.invoke(&test_instance, "getMyWithArgsList", &vec![InvocationArg::from(arr_instance)]).unwrap();

        let s: String = jvm.to_rust(i).unwrap();
        assert!(s == "abcdefghi");
    }

    #[test]
    fn variadic_int_method() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();

        let s1 = InvocationArg::try_from(1).unwrap();
        let s2 = InvocationArg::try_from(2).unwrap();
        let s3 = InvocationArg::try_from(3).unwrap();

        let arr_instance = jvm.create_java_array("java.lang.Integer", &vec![s1, s2, s3]).unwrap();

        let i = jvm.invoke(&test_instance, "addInts", &vec![InvocationArg::from(arr_instance)]).unwrap();

        let num: i32 = jvm.to_rust(i).unwrap();
        assert!(num == 6);
    }

    #[test]
    fn variadic_long_primitive_method() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let values: Vec<i64> = vec![1, 2, 3];
        let jargs: Vec<_> = values
            .into_iter()
            .map(|v| InvocationArg::try_from(v).unwrap().into_primitive().unwrap())
            .collect();

        let arr_instance = jvm.create_java_array("long", &jargs).unwrap();

        let _ = jvm.invoke_static("org.astonbitecode.j4rs.tests.MyTest", "useLongPrimitivesArray", &vec![InvocationArg::from(arr_instance)]).unwrap();
    }

    #[test]
    fn instance_invocation_chain_and_collect() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &vec![InvocationArg::try_from("string").unwrap()]).unwrap();

        let i1 = jvm.chain(&instance).unwrap()
            .invoke("appendToMyString", &vec![InvocationArg::try_from("_is_appended").unwrap()]).unwrap()
            .invoke("length", &[]).unwrap()
            .collect();


        let product: isize = jvm.to_rust(i1).unwrap();

        assert!(product == 18);
    }

    #[test]
    fn instance_invocation_chain_and_to_rust() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
        let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &vec![InvocationArg::try_from("string").unwrap()]).unwrap();

        let product: isize = jvm.into_chain(instance)
            .invoke("appendToMyString", &vec![InvocationArg::try_from("_is_appended").unwrap()]).unwrap()
            .invoke("length", &[]).unwrap()
            .to_rust().unwrap();

        assert!(product == 18);
    }

    #[test]
    fn static_invocation_chain_and_to_rust() {
        let jvm: Jvm = JvmBuilder::new()
            .build()
            .unwrap();

        let static_invocation = jvm.static_class("java.lang.System").unwrap();

        let _: isize = jvm.into_chain(static_invocation)
            .invoke("currentTimeMillis", &[]).unwrap()
            .to_rust().unwrap();
    }

    #[test]
    fn access_class_field_and_enum() {
        let jvm: Jvm = JvmBuilder::new()
            .build()
            .unwrap();

        let static_invocation = jvm.static_class("java.lang.System").unwrap();
        let field_instance_res = jvm.field(&static_invocation, "out");
        assert!(field_instance_res.is_ok());

        let access_mode_enum = jvm.static_class("java.nio.file.AccessMode").unwrap();
        let access_mode_write = jvm.field(&access_mode_enum, "WRITE");
        assert!(access_mode_write.is_ok());
    }

    #[test]
    fn java_hello_world() {
        let jvm: Jvm = JvmBuilder::new()
            .build()
            .unwrap();

        let system = jvm.static_class("java.lang.System").unwrap();
        let _ = jvm.into_chain(system)
            .field("out").unwrap()
            .invoke("println", &vec![InvocationArg::try_from("Hello World").unwrap()]).unwrap()
            .collect();
    }

    #[test]
    fn parent_interface_method() {
        let jvm: Jvm = JvmBuilder::new()
            .build()
            .unwrap();
        let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();

        let size: isize = jvm.into_chain(instance)
            .invoke("getMap", &[]).unwrap()
            .cast("java.util.Map").unwrap()
            .invoke("size", &[]).unwrap()
            .to_rust().unwrap();

        assert!(size == 2);
    }

    #[test]
    fn invoke_generic_method() {
        let jvm: Jvm = JvmBuilder::new()
            .build()
            .unwrap();

        // Create the MyTest instance
        let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();

        // Retrieve the annotated Map
        let dummy_map = jvm.invoke(&instance, "getMap", &[]).unwrap();

        // Put a new Map entry
        let _ = jvm.invoke(&dummy_map, "put", &vec![InvocationArg::try_from("three").unwrap(), InvocationArg::try_from(3).unwrap()]).unwrap();

        // Get the size of the new map and assert
        let size: isize = jvm.into_chain(dummy_map)
            .invoke("size", &[]).unwrap()
            .to_rust().unwrap();

        assert!(size == 3);
    }

    #[test]
    fn invoke_method_with_primitive_args() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();

        // Test the primitives in constructors.
        // The constructor of Integer takes a primitive int as an argument.
        let ia = InvocationArg::try_from(1_i32).unwrap().into_primitive().unwrap();
        let res1 = jvm.create_instance("java.lang.Integer", &[ia]);
        assert!(res1.is_ok());

        // Test the primitives in invocations.
        let ia1 = InvocationArg::try_from(1_i32).unwrap();
        let ia2 = InvocationArg::try_from(1_i32).unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
        let res2 = jvm.invoke(&test_instance, "addInts", &[ia1.into_primitive().unwrap(), ia2.into_primitive().unwrap()]);
        assert!(res2.is_ok());
    }

    #[test]
    fn to_tust_returns_list() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
        let list_instance = jvm.invoke(&test_instance, "getNumbersUntil", &[InvocationArg::try_from(10_i32).unwrap()]).unwrap();
        let vec: Vec<i32> = jvm.to_rust(list_instance).unwrap();
        assert!(vec.len() == 10)
    }

    #[test]
    fn basic_types() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();

        // By values
        let arg = InvocationArg::try_from(33_i8).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i8 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i8);

        let arg = InvocationArg::try_from(33_i16).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i16 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i16);

        let arg = InvocationArg::try_from(33_i32).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i32 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i32);

        let arg = InvocationArg::try_from(33_i64).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i64 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i64);

        let arg = InvocationArg::try_from(33.33_f32).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: f32 = jvm.to_rust(i).unwrap();
        assert!(ret == 33.33_f32);

        let arg = InvocationArg::try_from(33.33_f64).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: f64 = jvm.to_rust(i).unwrap();
        assert!(ret == 33.33_f64);

        // By reference
        let arg = InvocationArg::try_from(&33_i8).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i8 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i8);

        let arg = InvocationArg::try_from(&33_i16).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i16 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i16);

        let arg = InvocationArg::try_from(&33_i32).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i32 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i32);

        let arg = InvocationArg::try_from(&33_i64).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: i64 = jvm.to_rust(i).unwrap();
        assert!(ret == 33_i64);

        let arg = InvocationArg::try_from(&33.33_f32).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: f32 = jvm.to_rust(i).unwrap();
        assert!(ret == 33.33_f32);

        let arg = InvocationArg::try_from(&33.33_f64).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: f64 = jvm.to_rust(i).unwrap();
        assert!(ret == 33.33_f64);
    }

    #[test]
    fn vecs_arrays() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();

        let arg = InvocationArg::try_from([33_i8, 34_i8].as_slice()).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: Vec<i8> = jvm.to_rust(i).unwrap();
        assert!(ret == vec![33_i8, 34_i8]);

        let arg = InvocationArg::try_from([33_i16, 34_i16].as_slice()).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: Vec<i16> = jvm.to_rust(i).unwrap();
        assert!(ret == vec![33_i16, 34_i16]);

        let arg = InvocationArg::try_from([33_i32, 34_i32].as_slice()).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: Vec<i32> = jvm.to_rust(i).unwrap();
        assert!(ret == vec![33_i32, 34_i32]);

        let arg = InvocationArg::try_from([33_i64, 34_i64].as_slice()).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: Vec<i64> = jvm.to_rust(i).unwrap();
        assert!(ret == vec![33_i64, 34_i64]);

        let arg = InvocationArg::try_from([33_f32, 34_f32].as_slice()).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: Vec<f32> = jvm.to_rust(i).unwrap();
        assert!(ret == vec![33_f32, 34_f32]);

        let arg = InvocationArg::try_from([33_f64, 34_f64].as_slice()).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &[arg]).unwrap();
        let ret: Vec<f64> = jvm.to_rust(i).unwrap();
        assert!(ret == vec![33_f64, 34_f64]);
    }

    #[test]
    fn null_handling() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
        let null = jvm.invoke(&test_instance, "getNullInteger", &[]).unwrap();
        let list_instance = jvm.invoke(&test_instance, "getNumbersUntil", &[InvocationArg::from(null)]).unwrap();
        let vec: Vec<i32> = jvm.to_rust(list_instance).unwrap();
        assert!(vec.is_empty())
    }

    #[test]
    fn null_creation() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
        let null = InvocationArg::try_from(Null::Of("java.lang.Integer")).unwrap();
        let list_instance = jvm.invoke(&test_instance, "getNumbersUntil", &[null]).unwrap();
        let vec: Vec<i32> = jvm.to_rust(list_instance).unwrap();
        assert!(vec.is_empty());

        let null = InvocationArg::try_from(Null::Integer).unwrap();
        let list_instance = jvm.invoke(&test_instance, "getNumbersUntil", &[null]).unwrap();
        let vec: Vec<i32> = jvm.to_rust(list_instance).unwrap();
        assert!(vec.is_empty());
    }

    #[test]
    fn to_rust_boxed() {
        let jvm = JvmBuilder::new().build().unwrap();
        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();

        let i = jvm.invoke(&test_instance, "echo", &vec![InvocationArg::try_from(true).unwrap()]).unwrap();
        let _: Box<bool> = jvm.to_rust_boxed(i).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &vec![InvocationArg::try_from(33_i8).unwrap()]).unwrap();
        let _: Box<i8> = jvm.to_rust_boxed(i).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &vec![InvocationArg::try_from(33_i16).unwrap()]).unwrap();
        let _: Box<i16> = jvm.to_rust_boxed(i).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &vec![InvocationArg::try_from(33_i32).unwrap()]).unwrap();
        let _: Box<i32> = jvm.to_rust_boxed(i).unwrap();
        let i = jvm.invoke(&test_instance, "echo", &vec![InvocationArg::try_from(33_i64).unwrap()]).unwrap();
        let _: Box<i64> = jvm.to_rust_boxed(i).unwrap();
    }
}
