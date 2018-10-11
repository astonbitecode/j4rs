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

extern crate jni_sys;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub use api::Callback as Callback;
pub use api::ClasspathEntry as ClasspathEntry;
pub use api::Instance as Instance;
pub use api::InstanceReceiver as InstanceReceiver;
pub use api::InvocationArg as InvocationArg;
pub use api::JavaOpt as JavaOpt;
pub use api::Jvm as Jvm;
use jni_sys::{JNIEnv, jobject};
use logger::info;
use std::mem;
use std::os::raw::{c_long, c_void};
use std::sync::mpsc::Sender;

mod api;
mod utils;
mod logger;

pub mod errors;

// TODO: Seems that this is not needed
// Initialize the environment
include!(concat!(env!("OUT_DIR"), "/j4rs_init.rs"));

/// Creates a new JVM, using the provided classpath entries and JVM arguments
pub fn new_jvm(classpath_entries: Vec<ClasspathEntry>, java_opts: Vec<JavaOpt>) -> errors::Result<Jvm> {
    // The default classpath contains the j4rs
    let mut default_classpath_entry = std::env::current_exe()?;
    default_classpath_entry.pop();
    default_classpath_entry.push("jassets");
    default_classpath_entry.push("j4rs-0.1.6-jar-with-dependencies.jar");
    // Create a default classpath entry for the tests
    let mut tests_classpath_entry = std::env::current_exe()?;
    tests_classpath_entry.pop();
    tests_classpath_entry.pop();
    tests_classpath_entry.push("jassets");
    tests_classpath_entry.push("j4rs-0.1.6-jar-with-dependencies.jar");

    let default_class_path = format!("-Djava.class.path={}{}{}",
                                     default_classpath_entry
                                         .to_str()
                                         .unwrap_or("./jassets/j4rs-0.1.6-jar-with-dependencies.jar"),
                                     utils::classpath_sep(),
                                     tests_classpath_entry
                                         .to_str()
                                         .unwrap_or("./jassets/j4rs-0.1.6-jar-with-dependencies.jar"));

    let classpath = classpath_entries
        .iter()
        .fold(
            default_class_path,
            |all, elem| {
                format!("{}{}{}", all, utils::classpath_sep(), elem.to_string())
            });
    info(&format!("Setting classpath to {}", classpath));

    let default_library_path = utils::java_library_path()?;
    info(&format!("Setting library path to {}", default_library_path));

    // Populate the JVM Options
    let mut jvm_options = vec![classpath, default_library_path];
    java_opts.into_iter().for_each(|opt| jvm_options.push(opt.to_string()));

    Jvm::new(&jvm_options)
}

#[no_mangle]
pub extern fn Java_org_astonbitecode_j4rs_api_invocation_NativeCallbackSupport_docallback(_jni_env: *mut JNIEnv, _class: *const c_void, ptr_address: c_long, native_invocation: jobject) {
    let pointer_from_address = ptr_address as *const ();
    let function = unsafe {
        mem::transmute::<*const (), Callback>(pointer_from_address)
    };
    let jvm = Jvm::attach_thread().unwrap();
    function(jvm, Instance::from(native_invocation).unwrap());
}

#[no_mangle]
pub extern fn Java_org_astonbitecode_j4rs_api_invocation_NativeCallbackToRustChannelSupport_docallbacktochannel(_jni_env: *mut JNIEnv, _class: *const c_void, ptr_address: c_long, native_invocation: jobject) {
    let instance = Instance::from(native_invocation).unwrap();

    let p = ptr_address as *mut Sender<Instance>;
    let tx = unsafe { Box::from_raw(p) };

    let result = tx.send(instance);
    mem::forget(tx);
    result.unwrap();
}

#[cfg(test)]
mod lib_unit_tests {
    use std::{thread, time};
    use std::thread::JoinHandle;
    use super::{ClasspathEntry, Instance, InvocationArg, Jvm};

    #[test]
    fn create_instance_and_invoke() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();

        let instantiation_args = vec![InvocationArg::from("arg from Rust")];
        let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref());
        match instance {
            Ok(i) => {
                let invocation_args = vec![InvocationArg::from(" ")];
                let invocation_result = jvm.invoke(&i, "split", &invocation_args);
                assert!(invocation_result.is_ok());
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        };

        let instantiation_args_2 = vec![InvocationArg::from("arg from Rust")];
        let instance_2 = jvm.create_instance("java.lang.String", instantiation_args_2.as_ref());
        match instance_2 {
            Ok(i) => {
                let invocation_args = vec![InvocationArg::from(" ")];
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
    #[ignore]
    fn callback() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();

        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
            Ok(i) => {
                let res = jvm.invoke_async(&i, "performCallback", Vec::new().as_ref(), my_callback);
                let thousand_millis = time::Duration::from_millis(1000);
                thread::sleep(thousand_millis);
                assert!(res.is_ok());
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
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }
    }

    //#[test]
    //#[ignore]
    fn _memory_leaks() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();

        for i in 0..100000000 {
            match jvm.create_instance("org.astonbitecode.j4rs.tests.MySecondTest", Vec::new().as_ref()) {
                Ok(_) => {
                    if i % 100000 == 0 {
                        println!("{}", i);
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

    //#[test]
    //#[ignore]
    fn _memory_leaks_when_cloning_instances() {
        let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();

        for i in 0..100000000 {
            match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
                Ok(instance) => {
                    let i0 = instance.clone();
                    let i1 = instance.clone();
                    let i2 = instance.clone();
                    let i3 = instance.clone();
                    let i4 = instance.clone();
                    let i5 = instance.clone();
                    let i6 = instance.clone();
                    let i7 = instance.clone();
                    let i8 = instance.clone();
                    let i9 = instance.clone();

                    assert!(jvm.invoke(&i0, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i1, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i2, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i3, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i4, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i5, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i6, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i7, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i8, "aMethod", &[]).is_ok());
                    assert!(jvm.invoke(&i9, "aMethod", &[]).is_ok());
                    if i % 100000 == 0 {
                        println!("{}", i);
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

    #[test]
    fn cast() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], vec![]).unwrap();

        let instantiation_args = vec![InvocationArg::from("Hi")];
        let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
        jvm.cast(&instance, "java.lang.Object").unwrap();
    }

    #[test]
    fn invoke_vec() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], vec![]).unwrap();

        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
            Ok(i) => {
                let invocation_args = vec![InvocationArg::from((vec!["arg1", "arg2", "arg3", "arg33"].as_slice(), &jvm))];
                let _ = jvm.invoke(&i, "list", &invocation_args);
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
                    let instantiation_args = vec![InvocationArg::from(format!("Thread{}", i))];
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
        let instantiation_args = vec![InvocationArg::from("3")];
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
            let instantiation_args = vec![InvocationArg::from(tid.clone())];
            let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
            let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
            assert!(&tid == tid_from_java);
        }
        {
            let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
            let instantiation_args = vec![InvocationArg::from(tid.clone())];
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
                let instantiation_args = vec![InvocationArg::from(tid.clone())];
                let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
                let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
                assert!(&tid == tid_from_java);
            }
            {
                let jvm: Jvm = super::new_jvm(Vec::new(), Vec::new()).unwrap();
                let instantiation_args = vec![InvocationArg::from(tid.clone())];
                let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
                let ref tid_from_java: String = jvm.to_rust(instance).unwrap();
                assert!(&tid == tid_from_java);
            }
            true
        });

        assert!(jh.join().unwrap());
    }

    fn my_callback(jvm: Jvm, inst: Instance) {
        let string_from_java: String = jvm.to_rust(inst).unwrap();
        println!("Asynchronously got from Java: {}", string_from_java);
    }
}
