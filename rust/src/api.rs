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

use jni_sys::{
    self,
    JavaVM,
    JavaVMInitArgs,
    JavaVMOption,
    JNI_CreateJavaVM,
    JNI_ERR,
    JNI_EDETACHED,
    JNI_EVERSION,
    JNI_ENOMEM,
    JNI_EEXIST,
    JNI_EINVAL,
    JNI_FALSE,
    JNI_TRUE,
    JNI_OK,
    JNI_VERSION_1_8,
    JNIEnv,
    jobject,
    jobjectArray,
    jstring,
    jboolean,
    jclass,
    jmethodID,
    jsize,
    JNI_GetCreatedJavaVMs,
};
use super::logger::{debug, info};
use libc::c_char;
use ::errors;
use std::ptr;
use std::fs;
use std::os::raw::c_void;
use ::utils;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use std::sync::Mutex;

#[link(name = "jvm")]
extern {}

//type JniFindClass = unsafe extern "system" fn(env: *mut JNIEnv, name: *const c_char) -> jclass;
type JniGetMethodId = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, *const i8, *const i8) -> *mut jni_sys::_jmethodID;
type JniGetStaticMethodId = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, *const i8, *const i8) -> *mut jni_sys::_jmethodID;
#[allow(non_snake_case)]
type JniNewObject = unsafe extern "C" fn(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, ...) -> jobject;
type JniNewStringUTF = unsafe extern "system" fn(env: *mut JNIEnv, utf: *const c_char) -> jstring;
#[allow(non_snake_case)]
type JniGetStringUTFChars = unsafe extern "system" fn(env: *mut JNIEnv, str: jstring, isCopy: *mut jboolean) -> *const c_char;
#[allow(non_snake_case)]
type JniCallObjectMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
type JniCallStaticObjectMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
type JniNewObjectArray = unsafe extern "system" fn(env: *mut JNIEnv, len: jsize, clazz: jclass, init: jobject) -> jobjectArray;
type JniSetObjectArrayElement = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, i32, *mut jni_sys::_jobject);
type JniExceptionCheck = unsafe extern "system" fn(_: *mut JNIEnv) -> jboolean;
type JniExceptionDescribe = unsafe extern "system" fn(_: *mut JNIEnv);
type JniExceptionClear = unsafe extern "system" fn(_: *mut JNIEnv);
pub type Callback = fn(Jvm, Instance) -> ();

const RUST: &'static str = "rust";
const JAVA: &'static str = "java";
const INST_CLASS_NAME: &'static str = "org/astonbitecode/j4rs/api/instantiation/NativeInstantiationImpl";
const INVO_IFACE_NAME: &'static str = "org/astonbitecode/j4rs/api/NativeInvocation";
const UNKNOWN_FOR_RUST: &'static str = "known_in_java_world";
const J4RS_ARRAY: &'static str = "org.astonbitecode.j4rs.api.dtos.Array";

lazy_static! {
    // Synchronize the creation of Jvm
    static ref MUTEX: Mutex<bool> = Mutex::new(false);
}


/// Holds the assets for the JVM
#[derive(Clone)]
pub struct Jvm {
    //    _jvm: *mut JavaVM,
    jni_env: *mut JNIEnv,
    //    _jni_find_class: JniFindClass,
    jni_get_method_id: JniGetMethodId,
    jni_get_static_method_id: JniGetStaticMethodId,
    jni_new_object: JniNewObject,
    jni_new_string_utf: JniNewStringUTF,
    jni_get_string_utf_chars: JniGetStringUTFChars,
    jni_call_object_method: JniCallObjectMethod,
    jni_call_static_object_method: JniCallStaticObjectMethod,
    jni_new_onject_array: JniNewObjectArray,
    jni_set_object_array_element: JniSetObjectArrayElement,
    jni_exception_check: JniExceptionCheck,
    jni_exception_describe: JniExceptionDescribe,
    jni_exception_clear: JniExceptionClear,
    /// This is the factory class. It creates instances using reflection. Currently the `NativeInstantiationImpl`.
    factory_class: jclass,
    /// The constructor method of the `NativeInstantiationImpl`.
    factory_constructor_method: jmethodID,
    /// The method id of the `instantiate` method of the `NativeInvocation`.
    factory_instantiate_method: jmethodID,
    /// The method id of the `createForStatic` method of the `NativeInvocation`.
    factory_create_for_static_method: jmethodID,
    /// The `NativeInvocation` class.
    native_invocation_class: jclass,
    /// The Java class for the `InvocationArg`.
    invocation_arg_class: jclass,
}

impl Jvm {
    /// Creates a new Jvm. If a Jvm is already created by the current process, it returns the created Jvm.
    pub fn new(jvm_options: &[String]) -> errors::Result<Jvm> {
        info("Attempting to create a new JVM");
        let mut jvm: *mut JavaVM = ptr::null_mut();
        let mut jni_environment: *mut JNIEnv = ptr::null_mut();

        let _g = MUTEX.lock().unwrap();
        let created_vm = Jvm::get_created_vm();

        let result = if created_vm.is_some() {
            info("A JVM is already created. Retrieving it...");
            jni_environment = created_vm.unwrap();

            JNI_OK
        } else {
            info("No JVMs exist. Creating a new one...");
            let mut jvm_options_vec: Vec<JavaVMOption> = jvm_options
                .iter()
                .map(|opt| {
                    JavaVMOption {
                        optionString: utils::to_java_string(opt) as *mut i8,
                        extraInfo: ptr::null_mut() as *mut c_void,
                    }
                })
                .collect();

            let mut jvm_arguments = JavaVMInitArgs {
                version: JNI_VERSION_1_8,
                nOptions: jvm_options.len() as i32,
                options: jvm_options_vec.as_mut_ptr(),
                ignoreUnrecognized: JNI_FALSE,
            };

            unsafe {
                JNI_CreateJavaVM(
                    &mut jvm,
                    (&mut jni_environment as *mut *mut JNIEnv) as *mut *mut c_void,
                    (&mut jvm_arguments as *mut JavaVMInitArgs) as *mut c_void,
                )
            }
        };

        if result != JNI_OK {
            let error_message = match result {
                JNI_EDETACHED => "thread detached from the JVM",
                JNI_EEXIST => "JVM already created",
                JNI_EINVAL => "invalid arguments",
                JNI_ENOMEM => "not enough memory",
                JNI_ERR => "unknown error",
                JNI_EVERSION => "JNI version error",
                _ => "unknown JNI error value",
            };

            Err(errors::J4RsError::JavaError(format!("Could not create the JVM: {}", error_message).to_string()))
        } else {
            let jvm = Self::try_from(jni_environment)?;
            // Pass to the Java world the name of the j4rs library.
            let found_libs: Vec<String> = fs::read_dir(utils::deps_dir()?)?
                .filter(|entry| {
                    entry.is_ok()
                })
                .filter(|entry| {
                    let entry = entry.as_ref().unwrap();
                    let file_name = entry.file_name();
                    let file_name = file_name.to_str().unwrap();
                    file_name.contains("j4rs") && (
                        file_name.contains(".so") ||
                            file_name.contains(".dll") ||
                            file_name.contains(".dylib"))
                })
                .map(|entry| entry.
                    unwrap().
                    file_name().
                    to_str().
                    unwrap().
                    to_owned())
                .collect();

            if found_libs.len() > 0 {
                let a_lib = found_libs[0].clone().replace("lib", "");
                let dot_splitted: Vec<&str> = a_lib.split(".").collect();
                jvm.invoke_static("org.astonbitecode.j4rs.api.invocation.NativeCallbackSupport",
                                  "initialize",
                                  &vec![InvocationArg::from(dot_splitted[0])])?;

                Ok(jvm)
            } else {
                Err(errors::J4RsError::GeneralError(
                    format!("Could not find the j4rs lib in {}", utils::deps_dir()?)))
            }
        }
    }

    pub fn try_from(jni_environment: *mut JNIEnv) -> errors::Result<Jvm> {
        unsafe {
            match ((**jni_environment).FindClass,
                   (**jni_environment).GetMethodID,
                   (**jni_environment).GetStaticMethodID,
                   (**jni_environment).NewObject,
                   (**jni_environment).NewStringUTF,
                   (**jni_environment).GetStringUTFChars,
                   (**jni_environment).CallObjectMethod,
                   (**jni_environment).CallStaticObjectMethod,
                   (**jni_environment).NewObjectArray,
                   (**jni_environment).SetObjectArrayElement,
                   (**jni_environment).ExceptionCheck,
                   (**jni_environment).ExceptionDescribe,
                   (**jni_environment).ExceptionClear) {
                (Some(fc), Some(gmid), Some(gsmid), Some(no), Some(nsu), Some(gsuc), Some(com), Some(csom), Some(noa), Some(soae), Some(ec), Some(ed), Some(exclear)) => {
                    // This is the factory class. It creates instances using reflection. Currently the `NativeInstantiationImpl`
                    let factory_class: jclass = (fc)(
                        jni_environment,
                        utils::to_java_string(INST_CLASS_NAME),
                    );
                    // The constructor of `NativeInstantiationImpl`
                    let factory_constructor_method = (gmid)(
                        jni_environment,
                        factory_class,
                        utils::to_java_string("<init>"),
                        utils::to_java_string("()V"));
                    // The class of the `InvocationArg`
                    let invocation_arg_class = (fc)(
                        jni_environment,
                        utils::to_java_string("org/astonbitecode/j4rs/api/dtos/InvocationArg"),
                    );
                    // `NativeInvocation` assets
                    let instantiate_method_signature = format!(
                        "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                        INVO_IFACE_NAME);
                    let create_for_static_method_signature = format!(
                        "(Ljava/lang/String;)L{};",
                        INVO_IFACE_NAME);
                    // The method id of the `instantiate` method of the `NativeInvocation`
                    let factory_instantiate_method = (gmid)(
                        jni_environment,
                        factory_class,
                        utils::to_java_string("instantiate"),
                        utils::to_java_string(&instantiate_method_signature),
                    );
                    // The method id of the `createForStatic` method of the `NativeInvocation`
                    let factory_create_for_static_method = (gmid)(
                        jni_environment,
                        factory_class,
                        utils::to_java_string("createForStatic"),
                        utils::to_java_string(&create_for_static_method_signature),
                    );
                    // The `NativeInvocation class`
                    let native_invocation_class: jclass = (fc)(
                        jni_environment,
                        utils::to_java_string(INVO_IFACE_NAME),
                    );

                    if (ec)(jni_environment) == JNI_TRUE {
                        (ed)(jni_environment);
                        (exclear)(jni_environment);
                        Err(errors::J4RsError::JavaError("The VM cannot be started... Please check the logs.".to_owned()))
                    } else {
                        let jvm = Jvm {
                            jni_env: jni_environment,
                            jni_get_method_id: gmid,
                            jni_get_static_method_id: gsmid,
                            jni_new_object: no,
                            jni_new_string_utf: nsu,
                            jni_get_string_utf_chars: gsuc,
                            jni_call_object_method: com,
                            jni_call_static_object_method: csom,
                            jni_new_onject_array: noa,
                            jni_set_object_array_element: soae,
                            jni_exception_check: ec,
                            jni_exception_describe: ed,
                            jni_exception_clear: exclear,
                            factory_class: factory_class,
                            factory_constructor_method: factory_constructor_method,
                            factory_instantiate_method: factory_instantiate_method,
                            factory_create_for_static_method: factory_create_for_static_method,
                            native_invocation_class: native_invocation_class,
                            invocation_arg_class: invocation_arg_class,
                        };
                        Ok(jvm)
                    }
                }
                (_, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    Err(errors::J4RsError::JniError(format!("Could not initialize the JVM: Error while trying to retrieve JNI functions.")))
                }
            }
        }
    }

    /// Creates an `Instance` of the class `class_name`, passing an array of `InvocationArg`s to construct the instance.
    pub fn create_instance(&self, class_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Instantiating class {} using {} arguments", class_name, inv_args.len()));
        unsafe {
            // The factory instance
            let factory_instance = (self.jni_new_object)(
                self.jni_env,
                self.factory_class,
                self.factory_constructor_method,
            );
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(class_name),
            );
            // Factory invocation - rest of the arguments: Create a new objectarray of class InvocationArg
            let size = inv_args.len() as i32;
            let array_ptr = (self.jni_new_onject_array)(
                self.jni_env,
                size,
                self.invocation_arg_class,
                ptr::null_mut(),
            );
            // Factory invocation - rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java = inv_args[i as usize].as_java_ptr(self);
                // Set it in the array
                (self.jni_set_object_array_element)(
                    self.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
            }
            // Call the method of the factory that instantiates a new class of `class_name`.
            // This returns a NativeInvocation that acts like a proxy to the Java world.
            let native_invocation_instance = (self.jni_call_object_method)(
                self.jni_env,
                factory_instance,
                self.factory_instantiate_method,
                class_name_jstring,
                array_ptr,
            );

            // Create and return the Instance
            self.do_return(Instance { jinstance: native_invocation_instance, class_name: class_name.to_owned() })
        }
    }

    /// Invokes the method `method_name` of a created `Instance`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke(&self, instance: &Instance, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Invoking method {} of class {} using {} arguments", method_name, instance.class_name, inv_args.len()));
        unsafe {
            let invoke_method_signature = format!(
                "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME);
            // Get the method ID for the `NativeInvocation.invoke`
            let invoke_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("invoke"),
                utils::to_java_string(invoke_method_signature.as_ref()),
            );

            // First argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(method_name),
            );
            // Rest of the arguments: Create a new objectarray of class InvocationArg
            let size = inv_args.len() as i32;
            let array_ptr = (self.jni_new_onject_array)(
                self.jni_env,
                size,
                self.invocation_arg_class,
                ptr::null_mut(),
            );
            // Rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java = inv_args[i as usize].as_java_ptr(self);
                // Set it in the array
                (self.jni_set_object_array_element)(
                    self.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
            }

            // Call the method of the instance
            let native_invocation_instance = (self.jni_call_object_method)(
                self.jni_env,
                instance.jinstance,
                invoke_method,
                method_name_jstring,
                array_ptr,
            );

            // Create and return the Instance
            self.do_return(Instance { jinstance: native_invocation_instance, class_name: UNKNOWN_FOR_RUST.to_owned() })
        }
    }


    /// Invokes asynchronously the method `method_name` of a created `Instance`, passing an array of `InvocationArg`s.
    /// It returns void and the `Instance` of the result of the async invocation will come in the defined callback.
    pub fn invoke_async(&self, instance: &Instance, method_name: &str, inv_args: &[InvocationArg], callback: super::Callback) -> errors::Result<()> {
        debug(&format!("Asynchronously invoking method {} of class {} using {} arguments", method_name, instance.class_name, inv_args.len()));
        unsafe {
            let invoke_method_signature = "(JLjava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)V";
            // Get the method ID for the `NativeInvocation.invokeAsync`
            let invoke_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("invokeAsync"),
                utils::to_java_string(invoke_method_signature),
            );

            // First argument: the address of the callback function
            let address_string = format!("{:p}", callback as *const ());
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();
            // Second argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(method_name),
            );
            // Rest of the arguments: Create a new objectarray of class InvocationArg
            let size = inv_args.len() as i32;
            let array_ptr = (self.jni_new_onject_array)(
                self.jni_env,
                size,
                self.invocation_arg_class,
                ptr::null_mut(),
            );
            // Rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java = inv_args[i as usize].as_java_ptr(self);
                // Set it in the array
                (self.jni_set_object_array_element)(
                    self.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
            }

            // Call the method of the instance
            let _ = (self.jni_call_object_method)(
                self.jni_env,
                instance.jinstance,
                invoke_method,
                address,
                method_name_jstring,
                array_ptr,
            );

            // Create and return the Instance
            self.do_return(())
        }
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke_static(&self, class_name: &str, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Invoking static method {} of class {} using {} arguments", method_name, class_name, inv_args.len()));
        unsafe {
            // The factory instance
            let factory_instance = (self.jni_new_object)(
                self.jni_env,
                self.factory_class,
                self.factory_constructor_method,
            );
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(class_name),
            );
            // Call the method of the factory that creates a NativeInvocation for static calls to methods of class `class_name`.
            // This returns a NativeInvocation that acts like a proxy to the Java world.
            let native_invocation_instance = (self.jni_call_object_method)(
                self.jni_env,
                factory_instance,
                self.factory_create_for_static_method,
                class_name_jstring,
            );

            // The invokeStatic method signature
            let invoke_static_method_signature = format!(
                "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME);
            // Get the method ID for the `NativeInvocation.invokeStatic`
            let invoke_static_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("invokeStatic"),
                utils::to_java_string(invoke_static_method_signature.as_ref()),
            );

            // First argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(method_name),
            );
            // Rest of the arguments: Create a new objectarray of class InvocationArg
            let size = inv_args.len() as i32;
            let array_ptr = (self.jni_new_onject_array)(
                self.jni_env,
                size,
                self.invocation_arg_class,
                ptr::null_mut(),
            );
            // Rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java = inv_args[i as usize].as_java_ptr(self);
                // Set it in the array
                (self.jni_set_object_array_element)(
                    self.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
            }
            // Call the method of the instance
            let native_invocation_instance = (self.jni_call_object_method)(
                self.jni_env,
                native_invocation_instance,
                invoke_static_method,
                method_name_jstring,
                array_ptr,
            );

            // Create and return the Instance
            self.do_return(Instance::from(native_invocation_instance))
        }
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn cast(&self, from_instance: &Instance, to_class: &str) -> errors::Result<Instance> {
        debug(&format!("Casting to class {}", to_class));
        unsafe {
            // First argument is the jobject that is inside the from_instance
            // Second argument: create a jstring to pass as argument for the to_class
            let to_class_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(to_class),
            );

            // The cast method signature
            let cast_method_signature = format!(
                "(L{};Ljava/lang/String;)L{};",
                INVO_IFACE_NAME,
                INVO_IFACE_NAME);

            // Get the method ID for the `NativeInvocation.cast`
            let cast_static_method = (self.jni_get_static_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("cast"),
                utils::to_java_string(cast_method_signature.as_ref()),
            );

            // Call the cast method
            let native_invocation_instance = (self.jni_call_static_object_method)(
                self.jni_env,
                self.native_invocation_class,
                cast_static_method,
                from_instance.jinstance,
                to_class_jstring,
            );

            // Create and return the Instance
            self.do_return(Instance::from(native_invocation_instance))
        }
    }

    /// Returns the Rust representation of the provided instance
    pub fn to_rust<T>(&self, instance: Instance) -> errors::Result<T> where T: DeserializeOwned {
        unsafe {
            // The getJson method signature
            let get_json_method_signature = "()Ljava/lang/String;";

            // Get the method ID for the `NativeInvocation.cast`
            let get_json_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("getJson"),
                utils::to_java_string(get_json_method_signature.as_ref()),
            );

            // Call the getJson method
            let json_instance = (self.jni_call_object_method)(
                self.jni_env,
                instance.jinstance,
                get_json_method,
            );
            let _ = self.do_return("")?;
            let json = {
                let s = (self.jni_get_string_utf_chars)(
                    self.jni_env,
                    json_instance as jstring,
                    ptr::null_mut(),
                );
                let _ = self.do_return("")?;
                utils::to_rust_string(s)
            };
            self.do_return(serde_json::from_str(&json)?)
        }
    }

    fn do_return<T>(&self, to_return: T) -> errors::Result<T> {
        unsafe {
            if (self.jni_exception_check)(self.jni_env) == JNI_TRUE {
                (self.jni_exception_describe)(self.jni_env);
                (self.jni_exception_clear)(self.jni_env);
                Err(errors::J4RsError::JavaError("An Exception was thrown by Java... Please check the logs or the console.".to_owned()))
            } else {
                Ok(to_return)
            }
        }
    }

    fn get_created_vm() -> Option<*mut JNIEnv> {
        unsafe {
            // Get the number of the already created VMs. This is most probably 1, but we retrieve the number just in case...
            let mut created_vms_size: jsize = 0;
            JNI_GetCreatedJavaVMs(ptr::null_mut(), 0, &mut created_vms_size);

            if created_vms_size == 0 {
                None
            } else {
                info(&format!("Retrieving the first of {} created JVMs", created_vms_size));
                // Get the created VM
                let mut buffer: Vec<*mut JavaVM> = Vec::new();
                for _ in 0..created_vms_size { buffer.push(ptr::null_mut()); }

                let retjint = JNI_GetCreatedJavaVMs(buffer.as_mut_ptr(), created_vms_size, &mut created_vms_size);
                if retjint == JNI_OK {
                    match (**buffer[0]).AttachCurrentThread {
                        Some(act) => {
                            let mut jni_environment: *mut JNIEnv = ptr::null_mut();
                            (act)(
                                buffer[0],
                                (&mut jni_environment as *mut *mut JNIEnv) as *mut *mut c_void,
                                ptr::null_mut(),
                            );
                            Some(jni_environment)
                        }
                        None => {
                            println!("Cannot attach the thread to the JVM");
                            None
                        },
                    }
                } else {
                    println!("Error while retrieving the created JVMs: {}", retjint);
                    None
                }
            }
        }
    }
}

/// Struct that carries an argument that is used for method invocations in Java.
#[derive(Serialize)]
pub enum InvocationArg {
    /// An arg that is created in the Java world.
    Java {
        instance: Instance,
        class_name: String,
        arg_from: String,
    },
    // An arg that is created in the Rust world.
    Rust {
        json: String,
        class_name: String,
        arg_from: String,
    },
}

impl InvocationArg {
    /// Creates a InvocationArg::Rust.
    /// This is default for the Args that are created from the Rust code.
    pub fn new<T: ?Sized>(arg: &T, class_name: &str) -> InvocationArg
        where T: Serialize
    {
        let json = serde_json::to_string(arg).unwrap();
        InvocationArg::from((json.as_ref(), class_name))
    }

    /// Creates a `jobject` from this InvocationArg.
    pub fn as_java_ptr(&self, jvm: &Jvm) -> jobject {
        match self {
            _s @ &InvocationArg::Java { .. } => self.jobject_from_java(jvm),
            _s @ &InvocationArg::Rust { .. } => self.jobject_from_rust(jvm),
        }
    }

    fn jobject_from_rust(&self, jvm: &Jvm) -> jobject {
        unsafe {
            // The constructor of `InvocationArg` for Rust created args
            let inv_arg_rust_constructor_method = (jvm.jni_get_method_id)(
                jvm.jni_env,
                jvm.invocation_arg_class,
                utils::to_java_string("<init>"),
                utils::to_java_string("(Ljava/lang/String;Ljava/lang/String;)V"));

            let (class_name, json) = match self {
                _s @ &InvocationArg::Java { .. } => panic!("Called jobject_from_rust for an InvocationArg that is created by Java. Please consider opening a bug to the developers."),
                &InvocationArg::Rust { ref class_name, ref json, .. } => {
                    debug(&format!("Creating jobject from Rust for class {}", class_name));
                    (class_name.to_owned(), json.to_owned())
                }
            };

            debug(&format!("Calling the InvocationArg constructor with '{}' and '{}'", class_name, json));
            let inv_arg_instance = (jvm.jni_new_object)(
                jvm.jni_env,
                jvm.invocation_arg_class,
                inv_arg_rust_constructor_method,
                // First argument: class_name
                (jvm.jni_new_string_utf)(
                    jvm.jni_env,
                    utils::to_java_string(class_name.as_ref()),
                ),
                // Second argument: json
                (jvm.jni_new_string_utf)(
                    jvm.jni_env,
                    utils::to_java_string(json.as_ref()),
                ),
            );

            inv_arg_instance
        }
    }

    fn jobject_from_java(&self, jvm: &Jvm) -> jobject {
        unsafe {
            let signature = format!("(Ljava/lang/String;L{};)V", INVO_IFACE_NAME);
            // The constructor of `InvocationArg` for Java created args
            let inv_arg_java_constructor_method = (jvm.jni_get_method_id)(
                jvm.jni_env,
                jvm.invocation_arg_class,
                utils::to_java_string("<init>"),
                utils::to_java_string(&signature));

            let (class_name, jinstance) = match self {
                _s @ &InvocationArg::Rust { .. } => panic!("Called jobject_from_java for an InvocationArg that is created by Rust. Please consider opening a bug to the developers."),
                &InvocationArg::Java { ref class_name, ref instance, .. } => {
                    debug(&format!("Creating jobject from Java for class {}", class_name));
                    (class_name.to_owned(), instance.jinstance)
                }
            };

            debug(&format!("Calling the InvocationArg constructor with '{}'", class_name));

            let inv_arg_instance = (jvm.jni_new_object)(
                jvm.jni_env,
                jvm.invocation_arg_class,
                inv_arg_java_constructor_method,
                // First argument: class_name
                (jvm.jni_new_string_utf)(
                    jvm.jni_env,
                    utils::to_java_string(class_name.as_ref()),
                ),
                // Second argument: NativeInvocation instance
                jinstance,
            );

            inv_arg_instance
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for InvocationArg {
    fn from(tup: (&'a str, &'a str)) -> InvocationArg {
        InvocationArg::Rust {
            json: tup.0.to_owned(),
            class_name: tup.1.to_owned(),
            arg_from: RUST.to_owned(),
        }
    }
}

impl From<Instance> for InvocationArg {
    fn from(instance: Instance) -> InvocationArg {
        let class_name = instance.class_name.to_owned();

        InvocationArg::Java {
            instance: instance,
            class_name: class_name,
            arg_from: JAVA.to_owned(),
        }
    }
}

impl From<String> for InvocationArg {
    fn from(s: String) -> InvocationArg {
        InvocationArg::new(&s, "java.lang.String")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [String], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [String], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl<'a> From<&'a str> for InvocationArg {
    fn from(s: &str) -> InvocationArg {
        InvocationArg::new(s, "java.lang.String")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [&'a str], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [&'a str], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<bool> for InvocationArg {
    fn from(b: bool) -> InvocationArg {
        InvocationArg::new(&b, "java.lang.Boolean")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [bool], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [bool], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<i8> for InvocationArg {
    fn from(b: i8) -> InvocationArg {
        InvocationArg::new(&b, "java.lang.Byte")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [i8], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [i8], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<char> for InvocationArg {
    fn from(c: char) -> InvocationArg {
        InvocationArg::new(&c, "java.lang.Character")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [char], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [char], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<i16> for InvocationArg {
    fn from(i: i16) -> InvocationArg {
        InvocationArg::new(&i, "java.lang.Short")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [i16], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [i16], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<i32> for InvocationArg {
    fn from(i: i32) -> InvocationArg {
        InvocationArg::new(&i, "java.lang.Integer")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [i32], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [i32], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<i64> for InvocationArg {
    fn from(l: i64) -> InvocationArg {
        InvocationArg::new(&l, "java.lang.Long")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [i64], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [i64], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<f32> for InvocationArg {
    fn from(f: f32) -> InvocationArg {
        InvocationArg::new(&f, "java.lang.Float")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [f32], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [f32], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<f64> for InvocationArg {
    fn from(f: f64) -> InvocationArg {
        InvocationArg::new(&f, "java.lang.Double")
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b> From<(&'a [f64], &'b Jvm)> for InvocationArg {
    fn from(vec_t_tup: (&'a [f64], &'b Jvm)) -> InvocationArg {
        let (vec, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|&elem| InvocationArg::from(elem)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b, T> From<(&'a [T], &'a str, &'b Jvm)> for InvocationArg where T: Serialize {
    fn from(vec_t_tup: (&'a [T], &'a str, &'b Jvm)) -> InvocationArg {
        let (vec, elements_class_name, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|elem| InvocationArg::new(elem, elements_class_name)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(Instance::from(res.unwrap()))
    }
}

impl From<()> for InvocationArg {
    fn from(_: ()) -> InvocationArg {
        InvocationArg::new(&(), "void")
    }
}

impl<'a> From<&'a String> for InvocationArg {
    fn from(s: &String) -> InvocationArg {
        InvocationArg::new(s, "java.lang.String")
    }
}

impl<'a> From<&'a bool> for InvocationArg {
    fn from(b: &bool) -> InvocationArg {
        InvocationArg::new(b, "java.lang.Boolean")
    }
}

impl<'a> From<&'a i8> for InvocationArg {
    fn from(b: &i8) -> InvocationArg {
        InvocationArg::new(b, "java.lang.Byte")
    }
}

impl<'a> From<&'a char> for InvocationArg {
    fn from(c: &char) -> InvocationArg {
        InvocationArg::new(c, "java.lang.Character")
    }
}

impl<'a> From<&'a i16> for InvocationArg {
    fn from(i: &i16) -> InvocationArg {
        InvocationArg::new(i, "java.lang.Short")
    }
}

impl<'a> From<&'a i32> for InvocationArg {
    fn from(i: &i32) -> InvocationArg {
        InvocationArg::new(i, "java.lang.Integer")
    }
}

impl<'a> From<&'a i64> for InvocationArg {
    fn from(l: &i64) -> InvocationArg {
        InvocationArg::new(l, "java.lang.Long")
    }
}

impl<'a> From<&'a f32> for InvocationArg {
    fn from(f: &f32) -> InvocationArg {
        InvocationArg::new(f, "java.lang.Float")
    }
}

impl<'a> From<&'a f64> for InvocationArg {
    fn from(f: &f64) -> InvocationArg {
        InvocationArg::new(f, "java.lang.Double")
    }
}

/// A Java instance
#[derive(Serialize, Clone)]
pub struct Instance {
    /// The name of the class of this instance
    class_name: String,
    /// The JNI jobject that manipulates this instance.
    ///
    /// This object is an instance of `org/astonbitecode/j4rs/api/NativeInvocation`
    #[serde(skip)]
    jinstance: jobject,
}

impl Instance {
    /// Returns the class name of this instance
    pub fn class_name(&self) -> &str {
        self.class_name.as_ref()
    }
    /// Consumes the Instance and returns its jobject
    pub fn java_object(self) -> jobject {
        self.jinstance
    }
}

impl From<jobject> for Instance {
    fn from(obj: jobject) -> Instance {
        Instance { jinstance: obj, class_name: UNKNOWN_FOR_RUST.to_owned() }
    }
}

/// A classpath entry.
#[derive(Debug)]
pub struct ClasspathEntry<'a> (&'a str);

impl<'a> ClasspathEntry<'a> {
    pub fn new(classpath_entry: &str) -> ClasspathEntry {
        ClasspathEntry(classpath_entry)
    }
}

impl<'a> ToString for ClasspathEntry<'a> {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

/// A Java Option.
#[derive(Debug)]
pub struct JavaOpt<'a> (&'a str);

impl<'a> JavaOpt<'a> {
    pub fn new(java_opt: &str) -> JavaOpt {
        JavaOpt(java_opt)
    }
}

impl<'a> ToString for JavaOpt<'a> {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[cfg(test)]
mod api_unit_tests {
    use super::InvocationArg;
    use serde_json;

    #[test]
    fn new_invocation_arg() {
        let _ = InvocationArg::new("something", "somethingelse");

        let gr = GuiResponse::ProvidedPassword { password: "passs".to_string(), number: 1 };
        let json = serde_json::to_string(&gr).unwrap();
        println!("{:?}", json);
        let res: Result<GuiResponse, _> = serde_json::from_str(&json);
        println!("{:?}", res);
    }

    #[derive(Serialize, Deserialize, Debug)]
    enum GuiResponse {
        ProvidedPassword { password: String, number: usize }
    }

    #[test]
    fn from_primitive_types() {
        validate_type(InvocationArg::from("str"), "java.lang.String");
        validate_type(InvocationArg::from("str".to_string()), "java.lang.String");
        validate_type(InvocationArg::from(true), "java.lang.Boolean");
        validate_type(InvocationArg::from(1_i8), "java.lang.Byte");
        validate_type(InvocationArg::from('c'), "java.lang.Character");
        validate_type(InvocationArg::from(1_i16), "java.lang.Short");
        validate_type(InvocationArg::from(1_i64), "java.lang.Long");
        validate_type(InvocationArg::from(0.1_f32), "java.lang.Float");
        validate_type(InvocationArg::from(0.1_f64), "java.lang.Double");
        validate_type(InvocationArg::from(()), "void");

        validate_type(InvocationArg::from(&"str".to_string()), "java.lang.String");
        validate_type(InvocationArg::from(&true), "java.lang.Boolean");
        validate_type(InvocationArg::from(&1_i8), "java.lang.Byte");
        validate_type(InvocationArg::from(&'c'), "java.lang.Character");
        validate_type(InvocationArg::from(&1_i16), "java.lang.Short");
        validate_type(InvocationArg::from(&1_i64), "java.lang.Long");
        validate_type(InvocationArg::from(&0.1_f32), "java.lang.Float");
        validate_type(InvocationArg::from(&0.1_f64), "java.lang.Double");
    }

    fn validate_type(ia: InvocationArg, class: &str) {
        let b = match ia {
            _s @ InvocationArg::Java { .. } => false,
            InvocationArg::Rust { class_name, json: _, .. } => {
                class == class_name
            }
        };
        assert!(b);
    }
}