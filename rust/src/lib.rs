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
extern crate log;
extern crate jni_sys;
extern crate libc;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod api;
mod utils;

pub mod errors;

pub use api::Jvm as Jvm;
pub use api::InvocationArg as InvocationArg;
pub use api::Instance as Instance;
pub use api::ClasspathEntry as ClasspathEntry;
pub use api::JavaOpt as JavaOpt;
pub use api::Callback as Callback;

use jni_sys::{JNIEnv, jobject};
use std::os::raw::{c_void, c_long};
use std::mem;

// TODO: Seems that this is not needed
// Initialize the environment
include!(concat!(env!("OUT_DIR"), "/j4rs_init.rs"));

/// Creates a new JVM, using the provided classpath entries and JVM arguments
pub fn new_jvm(classpath_entries: Vec<ClasspathEntry>, java_opts: Vec<JavaOpt>) -> errors::Result<Jvm> {
    // The default classpath contains the j4rs
    let mut default_classpath_entry = std::env::current_exe()?;
    default_classpath_entry.pop();
    default_classpath_entry.push("jassets");
    default_classpath_entry.push("j4rs-0.1.3.jar");
    // Create a default classpath entry for the tests
    let mut tests_classpath_entry = std::env::current_exe()?;
    tests_classpath_entry.pop();
    tests_classpath_entry.pop();
    tests_classpath_entry.push("jassets");
    tests_classpath_entry.push("j4rs-0.1.3.jar");

    let default_class_path = format!("-Djava.class.path={}{}{}",
                                     default_classpath_entry
                                         .to_str()
                                         .unwrap_or("./jassets/j4rs-0.1.3.jar"),
                                     utils::classpath_sep(),
                                     tests_classpath_entry
                                         .to_str()
                                         .unwrap_or("./jassets/j4rs-0.1.3.jar"));

    let classpath = classpath_entries
        .iter()
        .fold(
            default_class_path,
            |all, elem| {
                format!("{}{}{}", all, utils::classpath_sep(), elem.to_string())
            });
    println!("Setting classpath to {}", classpath);

    let default_library_path = utils::java_library_path()?;
    println!("Setting library path to {}", default_library_path);

    // Populate the JVM Options
    let mut jvm_options = vec![classpath, default_library_path];
    java_opts.into_iter().for_each(|opt| jvm_options.push(opt.to_string()));

    Jvm::new(&jvm_options)
}

#[no_mangle]
pub extern fn Java_org_astonbitecode_j4rs_api_invocation_NativeCallbackSupport_docallback(jni_env: *mut JNIEnv, _class: *const c_void, ptr_address: c_long, native_invocation: jobject) {
    let pointer_from_address = ptr_address as *const ();
    let function = unsafe {
        mem::transmute::<*const (), Callback>(pointer_from_address)
    };
    function(Jvm::try_from(jni_env).unwrap(), Instance::from(native_invocation));
}

#[cfg(test)]
mod lib_unit_tests {
    use super::{Jvm, InvocationArg, ClasspathEntry, Instance};
    use std::{thread, time};

    #[test]
    #[ignore]
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
    fn callback() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], Vec::new()).unwrap();
        //        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], vec![]).unwrap();

        match jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", Vec::new().as_ref()) {
            Ok(i) => {
                let _ = jvm.invoke_async(&i, "performCallback", Vec::new().as_ref(), my_callback);
                let ten_millis = time::Duration::from_millis(1000);
                thread::sleep(ten_millis);
            }
            Err(error) => {
                panic!("ERROR when creating Instance: {:?}", error);
            }
        }
    }

    #[test]
    #[ignore]
    fn cast() {
        let jvm: Jvm = super::new_jvm(vec![ClasspathEntry::new("onemore.jar")], vec![]).unwrap();

        let instantiation_args = vec![InvocationArg::from("Hi")];
        let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
        jvm.cast(&instance, "java.lang.Object").unwrap();
    }

    #[test]
    #[ignore]
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

    fn my_callback(jvm: Jvm, inst: Instance) {
        let string_from_java: String = jvm.to_rust(inst).unwrap();
        println!("Asynchronously got from Java: {}", string_from_java);
    }

    /*
        #[test]
        fn dummy2() {
            let bfn = Box::new("this is a noxed string".to_string());
            println!("Original address: {:p}", &*bfn);
            let p = &*bfn;
            let address_string = format!("{:p}", p);
            println!("Original address: {}", address_string);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();
            println!("Dec address: {}", address);

            unsafe {
                let b = &*(address as *mut String);
    //            let b = Box::from_raw(address as *mut String);

                println!("{:?}", b);
            }
        }

        #[test]
        fn dummy3() {
            fn foo() -> i32 {
                println!("I'M IN");
                0
            }

            println!("Original address: {:p}", foo as *const ());
            let address_string = format!("{:p}", foo as *const ());
            println!("Original address string: {}", address_string);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();

            let pointer_from_address = address as *const ();
            println!("Address: {:p}", pointer_from_address);
            let function = unsafe {
                mem::transmute::<*const (), fn() -> i32>(pointer_from_address)
            };
            assert_eq!(function(), 0);
        }

        #[test]
        #[ignore]
        fn dummy4() {
            let foo = move |i: i32| {
                println!("I'M IN");
                i * 2
            };

            let boxed_foo = Box::new(foo);
            let boxed_boxed_foo = Box::new(boxed_foo);

            println!("Original address: {:p}", &boxed_boxed_foo);
            let address_string = format!("{:p}", &boxed_boxed_foo);
            println!("Original address string: {}", address_string);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();

            let pointer_from_address = address as *const ();
            println!("Address: {:p}", pointer_from_address);
            let function = unsafe {
                mem::transmute::<*const (), &FnOnce(i32) -> i32>(pointer_from_address)
            };
            (function)(3);
        }*/
}
