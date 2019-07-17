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

use std::{fs, mem};
use std::any::Any;
use std::cell::RefCell;
use std::ops::Drop;
use std::os::raw::c_void;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use fs_extra::dir::get_dir_content;
use jni_sys::{
    self,
    JavaVM,
    JavaVMInitArgs,
    JavaVMOption,
    jboolean,
    jclass,
    jint,
    jmethodID,
    JNI_EDETACHED,
    JNI_EEXIST,
    JNI_EINVAL,
    JNI_ENOMEM,
    JNI_ERR,
    JNI_EVERSION,
    JNI_FALSE,
    JNI_OK,
    JNI_TRUE,
    JNI_VERSION_1_8,
    JNIEnv,
    jobject,
    jobjectArray,
    jobjectRefType,
    jsize,
    jstring,
};
use libc::c_char;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

use crate::api_tweaks as tweaks;
use crate::errors;
use crate::errors::J4RsError;
use crate::utils;

use super::logger::{debug, error, info, warn};

// Initialize the environment
include!(concat!(env!("OUT_DIR"), "/j4rs_init.rs"));

type JniGetMethodId = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, *const c_char, *const c_char) -> *mut jni_sys::_jmethodID;
type JniGetStaticMethodId = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, *const c_char, *const c_char) -> *mut jni_sys::_jmethodID;
#[allow(non_snake_case)]
type JniNewObject = unsafe extern "C" fn(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, ...) -> jobject;
type JniNewStringUTF = unsafe extern "system" fn(env: *mut JNIEnv, utf: *const c_char) -> jstring;
#[allow(non_snake_case)]
type JniGetStringUTFChars = unsafe extern "system" fn(env: *mut JNIEnv, str: jstring, isCopy: *mut jboolean) -> *const c_char;
#[allow(non_snake_case)]
type JniCallObjectMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
#[allow(non_snake_case)]
type JniCallVoidMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...);
type JniCallStaticObjectMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
type JniNewObjectArray = unsafe extern "system" fn(env: *mut JNIEnv, len: jsize, clazz: jclass, init: jobject) -> jobjectArray;
type JniSetObjectArrayElement = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, i32, *mut jni_sys::_jobject);
type JniExceptionCheck = unsafe extern "system" fn(_: *mut JNIEnv) -> jboolean;
type JniExceptionDescribe = unsafe extern "system" fn(_: *mut JNIEnv);
type JniExceptionClear = unsafe extern "system" fn(_: *mut JNIEnv);
type JniDeleteLocalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> ();
type JniDeleteGlobalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> ();
type JniNewGlobalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> jobject;
pub type Callback = fn(Jvm, Instance) -> ();

const RUST: &'static str = "rust";
const JAVA: &'static str = "java";
const INST_CLASS_NAME: &'static str = "org/astonbitecode/j4rs/api/instantiation/NativeInstantiationImpl";
const INVO_BASE_NAME: &'static str = "org/astonbitecode/j4rs/api/NativeInvocationBase";
const INVO_IFACE_NAME: &'static str = "org/astonbitecode/j4rs/api/NativeInvocation";
const UNKNOWN_FOR_RUST: &'static str = "known_in_java_world";
const J4RS_ARRAY: &'static str = "org.astonbitecode.j4rs.api.dtos.Array";

lazy_static! {
    // Synchronize the creation of Jvm
    static ref MUTEX: Mutex<bool> = Mutex::new(false);
    // If a Jvm is created with defining a jassets_path other than the default, this is set here
    pub(crate) static ref JASSETS_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

thread_local! {
    static JNI_ENV: RefCell<Option<*mut JNIEnv>> = RefCell::new(None);
    static JVM: RefCell<Option<Jvm>> = RefCell::new(None);
    static ACTIVE_JVMS: RefCell<i32> = RefCell::new(0);
}

fn add_active_jvm() {
    ACTIVE_JVMS.with(|active_jvms| {
        let active_number = {
            *active_jvms.borrow() + 1
        };
        *active_jvms.borrow_mut() = active_number;
    });
}

fn remove_active_jvm() -> i32 {
    ACTIVE_JVMS.with(|active_jvms| {
        let active_number = {
            *active_jvms.borrow() - 1
        };
        *active_jvms.borrow_mut() = active_number;
        active_number
    })
}

fn set_thread_local_env(jni_env_opt: Option<*mut JNIEnv>) {
    JNI_ENV.with(|existing_jni_env_opt| {
        *existing_jni_env_opt.borrow_mut() = jni_env_opt;
    });
}

fn get_thread_local_env_opt() -> Option<*mut JNIEnv> {
    JNI_ENV.with(|existing_jni_env_opt| {
        match *existing_jni_env_opt.borrow() {
            Some(env) => Some(env.clone()),
            None => None,
        }
    })
}

fn get_thread_local_env() -> errors::Result<*mut JNIEnv> {
    match get_thread_local_env_opt() {
        Some(env) => Ok(env.clone()),
        None => Err(errors::J4RsError::JavaError(format!("Could not find the JNIEnv in the thread local"))),
    }
}

/// Holds the assets for the JVM
#[derive(Clone)]
pub struct Jvm {
    jni_env: *mut JNIEnv,
    jni_get_method_id: JniGetMethodId,
    jni_get_static_method_id: JniGetStaticMethodId,
    jni_new_object: JniNewObject,
    jni_new_string_utf: JniNewStringUTF,
    jni_get_string_utf_chars: JniGetStringUTFChars,
    jni_call_object_method: JniCallObjectMethod,
    jni_call_void_method: JniCallVoidMethod,
    jni_call_static_object_method: JniCallStaticObjectMethod,
    jni_new_onject_array: JniNewObjectArray,
    jni_set_object_array_element: JniSetObjectArrayElement,
    jni_exception_check: JniExceptionCheck,
    jni_exception_describe: JniExceptionDescribe,
    jni_exception_clear: JniExceptionClear,
    jni_delete_local_ref: JniDeleteLocalRef,
    jni_delete_global_ref: JniDeleteGlobalRef,
    jni_new_global_ref: JniNewGlobalRef,
    /// This is the factory class. It creates instances using reflection. Currently the `NativeInstantiationImpl`.
    factory_class: jclass,
    /// The constructor method of the `NativeInstantiationImpl`.
    factory_constructor_method: jmethodID,
    /// The method id of the `instantiate` method of the `NativeInstantiation`.
    factory_instantiate_method: jmethodID,
    /// The method id of the `createForStatic` method of the `NativeInstantiation`.
    factory_create_for_static_method: jmethodID,
    /// The method id of the `createJavaArray` method of the `NativeInstantiation`.
    factory_create_java_array_method: jmethodID,
    /// The `NativeInvocationBase` class.
    /// This is optional because it exists only in Android for Java7 compatibility
    /// because Java7 does not support static method implementations in interfaces.
    native_invocation_base_class: Option<jclass>,
    /// The `NativeInvocation` class.
    native_invocation_class: jclass,
    /// The Java class for the `InvocationArg`.
    invocation_arg_class: jclass,
    detach_thread_on_drop: bool,
}

impl Jvm {
    /// Creates a new Jvm.
    pub fn new(jvm_options: &[String], lib_name_to_load: Option<String>) -> errors::Result<Jvm> {
        Self::create_jvm(jvm_options, lib_name_to_load)
    }

    /// Attaches the current thread to an active JavaVM
    pub fn attach_thread() -> errors::Result<Jvm> {
        Self::create_jvm(&Vec::new(), None)
    }

    /// If true, the thread will not be detached when the Jvm is eing dropped.
    /// This is useful when creating a Jvm while on a Thread that is created in the Java world.
    /// When this Jvm is dropped, we don't want to detach the thread from the Java VM.
    ///
    /// It prevents errors like: `attempting to detach while still running code`
    pub fn detach_thread_on_drop(&mut self, detach: bool) {
        self.detach_thread_on_drop = detach;
    }

    /// Creates a new Jvm.
    /// If a JavaVM is already created by the current process, it attempts to attach the current thread to it.
    fn create_jvm(jvm_options: &[String], lib_name_to_load: Option<String>) -> errors::Result<Jvm> {
        debug("Creating a Jvm");
        let mut jvm: *mut JavaVM = ptr::null_mut();
        let mut jni_environment: *mut JNIEnv = ptr::null_mut();

        // Create the Jvm atomically
        let _g = MUTEX.lock()?;

        let result = if let Some(env) = get_thread_local_env_opt() {
            info("A JVM is already created for this thread. Retrieving it...");
            jni_environment = env;

            JNI_OK
        } else {
            let created_vm = Self::get_created_vm();

            let res_int = if created_vm.is_some() {
                debug("A JVM is already created by another thread. Retrieving it...");
                jni_environment = created_vm.unwrap();

                JNI_OK
            } else {
                info("No JVMs exist. Creating a new one...");
                let mut jvm_options_vec: Vec<JavaVMOption> = jvm_options
                    .iter()
                    .map(|opt| {
                        JavaVMOption {
                            optionString: utils::to_java_string(opt),
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

                tweaks::create_java_vm(
                    &mut jvm,
                    (&mut jni_environment as *mut *mut JNIEnv) as *mut *mut c_void,
                    (&mut jvm_arguments as *mut JavaVMInitArgs) as *mut c_void,
                )
            };

            res_int
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
            if let Some(libname) = lib_name_to_load {
                // Pass to the Java world the name of the j4rs library.
                debug(&format!("Initializing NativeCallbackSupport with libname {}", libname));
                jvm.invoke_static("org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport",
                                  "initialize",
                                  &vec![InvocationArg::from(libname)])?;
            }

            Ok(jvm)
        }
    }

    pub fn try_from(jni_environment: *mut JNIEnv) -> errors::Result<Jvm> {
        unsafe {
            match ((**jni_environment).GetMethodID,
                   (**jni_environment).GetStaticMethodID,
                   (**jni_environment).NewObject,
                   (**jni_environment).NewStringUTF,
                   (**jni_environment).GetStringUTFChars,
                   (**jni_environment).CallObjectMethod,
                   (**jni_environment).CallVoidMethod,
                   (**jni_environment).CallStaticObjectMethod,
                   (**jni_environment).NewObjectArray,
                   (**jni_environment).SetObjectArrayElement,
                   (**jni_environment).ExceptionCheck,
                   (**jni_environment).ExceptionDescribe,
                   (**jni_environment).ExceptionClear,
                   (**jni_environment).DeleteLocalRef,
                   (**jni_environment).DeleteGlobalRef,
                   (**jni_environment).NewGlobalRef) {
                (Some(gmid), Some(gsmid), Some(no), Some(nsu), Some(gsuc), Some(com), Some(cvm), Some(csom), Some(noa), Some(soae), Some(ec), Some(ed), Some(exclear), Some(dlr), Some(dgr), Some(ngr)) => {
                    // This is the factory class. It creates instances using reflection. Currently the `NativeInstantiationImpl`
                    let factory_class = tweaks::find_class(jni_environment, INST_CLASS_NAME);
                    // The constructor of `NativeInstantiationImpl`
                    let factory_constructor_method = (gmid)(
                        jni_environment,
                        factory_class,
                        utils::to_java_string("<init>"),
                        utils::to_java_string("()V"));
                    // The class of the `InvocationArg`
                    let invocation_arg_class = tweaks::find_class(
                        jni_environment,
                        "org/astonbitecode/j4rs/api/dtos/InvocationArg",
                    );
                    let instantiate_method_signature = format!(
                        "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                        INVO_IFACE_NAME);
                    let create_for_static_method_signature = format!(
                        "(Ljava/lang/String;)L{};",
                        INVO_IFACE_NAME);
                    let create_java_array_method_signature = format!(
                        "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                        INVO_IFACE_NAME);
                    // The method id of the `instantiate` method of the `NativeInstantiation`
                    let factory_instantiate_method = (gsmid)(
                        jni_environment,
                        factory_class,
                        utils::to_java_string("instantiate"),
                        utils::to_java_string(&instantiate_method_signature),
                    );
                    // The method id of the `createForStatic` method of the `NativeInstantiation`
                    let factory_create_for_static_method = (gsmid)(
                        jni_environment,
                        factory_class,
                        utils::to_java_string("createForStatic"),
                        utils::to_java_string(&create_for_static_method_signature),
                    );
                    // The method id of the `createJavaArray` method of the `NativeInstantiation`
                    let factory_create_java_array_method = (gsmid)(
                        jni_environment,
                        factory_class,
                        utils::to_java_string("createJavaArray"),
                        utils::to_java_string(&create_java_array_method_signature),
                    );
                    // The `NativeInvocationBase class`
                    let optional_class = if cfg!(target_os = "android") {
                        let native_invocation_base_class: jclass = tweaks::find_class(
                            jni_environment,
                            INVO_BASE_NAME,
                        );
                        Some(native_invocation_base_class)
                    } else {
                        None
                    };
                    // The `NativeInvocation class`
                    let native_invocation_class: jclass = tweaks::find_class(
                        jni_environment,
                        INVO_IFACE_NAME,
                    );

                    if (ec)(jni_environment) == JNI_TRUE {
                        (ed)(jni_environment);
                        (exclear)(jni_environment);
                        Err(errors::J4RsError::JavaError("The VM cannot be started... Please check the logs.".to_string()))
                    } else {
                        let jvm = Jvm {
                            jni_env: jni_environment,
                            jni_get_method_id: gmid,
                            jni_get_static_method_id: gsmid,
                            jni_new_object: no,
                            jni_new_string_utf: nsu,
                            jni_get_string_utf_chars: gsuc,
                            jni_call_object_method: com,
                            jni_call_void_method: cvm,
                            jni_call_static_object_method: csom,
                            jni_new_onject_array: noa,
                            jni_set_object_array_element: soae,
                            jni_exception_check: ec,
                            jni_exception_describe: ed,
                            jni_exception_clear: exclear,
                            jni_delete_local_ref: dlr,
                            jni_delete_global_ref: dgr,
                            jni_new_global_ref: ngr,
                            factory_class: factory_class,
                            factory_constructor_method: factory_constructor_method,
                            factory_instantiate_method: factory_instantiate_method,
                            factory_create_for_static_method: factory_create_for_static_method,
                            factory_create_java_array_method: factory_create_java_array_method,
                            native_invocation_base_class: optional_class,
                            native_invocation_class: native_invocation_class,
                            invocation_arg_class: invocation_arg_class,
                            detach_thread_on_drop: true,
                        };

                        if get_thread_local_env_opt().is_none() {
                            set_thread_local_env(Some(jni_environment));
                        }
                        add_active_jvm();

                        Ok(jvm)
                    }
                }
                (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
                    Err(errors::J4RsError::JniError(format!("Could not initialize the JVM: Error while trying to retrieve JNI functions.")))
                }
            }
        }
    }

    /// Creates an `Instance` of the class `class_name`, passing an array of `InvocationArg`s to construct the instance.
    pub fn create_instance(&self, class_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Instantiating class {} using {} arguments", class_name, inv_args.len()));
        unsafe {
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
            let native_invocation_instance = (self.jni_call_static_object_method)(
                self.jni_env,
                self.factory_class,
                self.factory_instantiate_method,
                class_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            self.do_return(())?;

            let native_invocation_global_instance = create_global_ref_from_local_ref(native_invocation_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            delete_java_local_ref(self.jni_env, array_ptr);
            delete_java_local_ref(self.jni_env, class_name_jstring);

            // Create and return the Instance
            self.do_return(Instance {
                jinstance: native_invocation_global_instance,
                class_name: class_name.to_string(),
            })
        }
    }

    /// Retrieves the static class `class_name`.
    pub fn static_class(&self, class_name: &str) -> errors::Result<Instance> {
        debug(&format!("Retrieving static class {}", class_name));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(class_name),
            );
            // Call the method of the factory that creates a NativeInvocation for static calls to methods of class `class_name`.
            // This returns a NativeInvocation that acts like a proxy to the Java world.
            let native_invocation_instance = (self.jni_call_static_object_method)(
                self.jni_env,
                self.factory_class,
                self.factory_create_for_static_method,
                class_name_jstring,
            );

            delete_java_local_ref(self.jni_env, class_name_jstring);

            let native_invocation_global_instance = create_global_ref_from_local_ref(native_invocation_instance, self.jni_env)?;

            // Create and return the Instance
            self.do_return(Instance::from(native_invocation_global_instance)?)
        }
    }

    /// Creates a new Java Array with elements of the class `class_name`.
    /// The array will have the `InvocationArg`s populated.
    /// The `InvocationArg`s __must__ be of type _class_name_.
    pub fn create_java_array(&self, class_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Creating a java array of class {} with {} elements", class_name, inv_args.len()));
        unsafe {
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
            // Call the method of the factory that instantiates a new Java Array of `class_name`.
            // This returns a NativeInvocation that acts like a proxy to the Java world.
            let native_invocation_instance = (self.jni_call_static_object_method)(
                self.jni_env,
                self.factory_class,
                self.factory_create_java_array_method,
                class_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            self.do_return(())?;

            let native_invocation_global_instance = create_global_ref_from_local_ref(native_invocation_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            delete_java_local_ref(self.jni_env, array_ptr);
            delete_java_local_ref(self.jni_env, class_name_jstring);

            // Create and return the Instance
            self.do_return(Instance {
                jinstance: native_invocation_global_instance,
                class_name: class_name.to_string(),
            })
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

            // Check for exceptions before creating the globalref
            self.do_return(())?;

            let native_invocation_global_instance = create_global_ref_from_local_ref(native_invocation_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            delete_java_local_ref(self.jni_env, array_ptr);
            delete_java_local_ref(self.jni_env, method_name_jstring);

            // Create and return the Instance
            self.do_return(Instance {
                jinstance: native_invocation_global_instance,
                class_name: UNKNOWN_FOR_RUST.to_string(),
            })
        }
    }

    /// Retrieves the field `field_name` of a created `Instance`.
    pub fn field(&self, instance: &Instance, field_name: &str) -> errors::Result<Instance> {
        debug(&format!("Retrieving field {} of class {}", field_name, instance.class_name));
        unsafe {
            let invoke_method_signature = format!(
                "(Ljava/lang/String;)L{};",
                INVO_IFACE_NAME);
            // Get the method ID for the `NativeInvocation.field`
            let field_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("field"),
                utils::to_java_string(invoke_method_signature.as_ref()),
            );

            // First argument: create a jstring to pass as argument for the field_name
            let field_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(field_name),
            );

            // Call the method of the instance
            let native_invocation_instance = (self.jni_call_object_method)(
                self.jni_env,
                instance.jinstance,
                field_method,
                field_name_jstring,
            );

            // Check for exceptions before creating the globalref
            self.do_return(())?;

            let native_invocation_global_instance = create_global_ref_from_local_ref(native_invocation_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            delete_java_local_ref(self.jni_env, field_name_jstring);

            // Create and return the Instance
            self.do_return(Instance {
                jinstance: native_invocation_global_instance,
                class_name: UNKNOWN_FOR_RUST.to_string(),
            })
        }
    }

    /// Invokes the method `method_name` of a created `Instance`, passing an array of `InvocationArg`s.
    /// It returns a Result of `InstanceReceiver` that may be used to get an underlying `Receiver<Instance>`. The result of the invocation will come via this Receiver.
    pub fn invoke_to_channel(&self, instance: &Instance, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<InstanceReceiver> {
        debug(&format!("Invoking method {} of class {} using {} arguments. The result of the invocation will come via an InstanceReceiver", method_name, instance.class_name, inv_args.len()));
        unsafe {
            let invoke_method_signature = "(JLjava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)V";
            // Get the method ID for the `NativeInvocation.invokeToChannel`
            let invoke_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("invokeToChannel"),
                utils::to_java_string(invoke_method_signature),
            );

            // Create the channel
            let (sender, rx) = channel();
            let tx = Box::new(sender);
            // First argument: the address of the channel Sender
            let raw_ptr = Box::into_raw(tx);
            // Find the address of tx
            let address_string = format!("{:p}", raw_ptr);
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
            let _ = (self.jni_call_void_method)(
                self.jni_env,
                instance.jinstance,
                invoke_method,
                address,
                method_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            self.do_return(())?;

            // Prevent memory leaks from the created local references
            delete_java_local_ref(self.jni_env, array_ptr);
            delete_java_local_ref(self.jni_env, method_name_jstring);

            // Create and return the Instance
            self.do_return(InstanceReceiver::new(rx, address))
        }
    }

    pub fn init_callback_channel(&self, instance: &Instance) -> errors::Result<InstanceReceiver> {
        debug(&format!("Initializing callback channel"));
        unsafe {
            let invoke_method_signature = "(J)V";
            // Get the method ID for the `NativeInvocation.initializeCallbackChannel`
            let invoke_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("initializeCallbackChannel"),
                utils::to_java_string(invoke_method_signature),
            );

            // Create the channel
            let (sender, rx) = channel();
            let tx = Box::new(sender);
            // First argument: the address of the channel Sender
            let raw_ptr = Box::into_raw(tx);
            // Find the address of tx
            let address_string = format!("{:p}", raw_ptr);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();

            // Call the method of the instance
            let _ = (self.jni_call_void_method)(
                self.jni_env,
                instance.jinstance,
                invoke_method,
                address,
            );

            // Create and return the Instance
            self.do_return(InstanceReceiver::new(rx, address))
        }
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke_static(&self, class_name: &str, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Invoking static method {} of class {} using {} arguments", method_name, class_name, inv_args.len()));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = (self.jni_new_string_utf)(
                self.jni_env,
                utils::to_java_string(class_name),
            );
            // Call the method of the factory that creates a NativeInvocation for static calls to methods of class `class_name`.
            // This returns a NativeInvocation that acts like a proxy to the Java world.
            let native_invocation_instance = (self.jni_call_static_object_method)(
                self.jni_env,
                self.factory_class,
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

            // Check for exceptions before creating the globalref
            self.do_return(())?;

            // Prevent memory leaks from the created local references
            delete_java_local_ref(self.jni_env, array_ptr);
            delete_java_local_ref(self.jni_env, method_name_jstring);

            let native_invocation_global_instance = create_global_ref_from_local_ref(native_invocation_instance, self.jni_env)?;

            // Create and return the Instance
            self.do_return(Instance::from(native_invocation_global_instance)?)
        }
    }

    /// Creates a clone of the provided Instance
    pub fn clone_instance(&self, instance: &Instance) -> errors::Result<Instance> {
        unsafe {
            // First argument is the jobject that is inside the instance

            // The clone method signature
            let clone_method_signature = format!(
                "(L{};)L{};",
                INVO_IFACE_NAME,
                INVO_IFACE_NAME);

            // The class to invoke the cloneInstance into is not the same in Android target os.
            // The native_invocation_base_class is checked first because of Java7 compatibility issues in Android.
            // In Java8 and later, the static implementation in the interfaces is used. This is not supported in Java7
            // and there is a base class created for this reason.
            let class_to_invoke_clone_instance = self.native_invocation_base_class.unwrap_or(self.native_invocation_class);

            // Get the method ID for the `NativeInvocation.clone`
            let clone_static_method = (self.jni_get_static_method_id)(
                self.jni_env,
                class_to_invoke_clone_instance,
                utils::to_java_string("cloneInstance"),
                utils::to_java_string(clone_method_signature.as_ref()),
            );

            // Call the clone method
            let native_invocation_instance = (self.jni_call_static_object_method)(
                self.jni_env,
                class_to_invoke_clone_instance,
                clone_static_method,
                instance.jinstance,
            );

            // Create and return the Instance
            self.do_return(Instance::from(native_invocation_instance)?)
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

            // The class to invoke the cast into is not the same in Android target os.
            // The native_invocation_base_class is checked first because of Java7 compatibility issues in Android.
            // In Java8 and later, the static implementation in the interfaces is used. This is not supported in Java7
            // and there is a base class created for this reason.
            let class_to_invoke_cast = self.native_invocation_base_class.unwrap_or(self.native_invocation_class);

            // Get the method ID for the `NativeInvocation.cast`
            let cast_static_method = (self.jni_get_static_method_id)(
                self.jni_env,
                class_to_invoke_cast,
                utils::to_java_string("cast"),
                utils::to_java_string(cast_method_signature.as_ref()),
            );

            // Call the cast method
            let native_invocation_instance = (self.jni_call_static_object_method)(
                self.jni_env,
                class_to_invoke_cast,
                cast_static_method,
                from_instance.jinstance,
                to_class_jstring,
            );

            // Check for exceptions before creating the globalref
            self.do_return(())?;

            // Prevent memory leaks from the created local references
            delete_java_local_ref(self.jni_env, to_class_jstring);

            // Create and return the Instance
            self.do_return(Instance::from(native_invocation_instance)?)
        }
    }

    /// Returns the Rust representation of the provided instance
    pub fn to_rust<T>(&self, instance: Instance) -> errors::Result<T> where T: DeserializeOwned {
        unsafe {
            debug("to_rust called");
            // The getJson method signature
            let get_json_method_signature = "()Ljava/lang/String;";

            // Get the method ID for the `NativeInvocation.getJson`
            let get_json_method = (self.jni_get_method_id)(
                self.jni_env,
                self.native_invocation_class,
                utils::to_java_string("getJson"),
                utils::to_java_string(get_json_method_signature.as_ref()),
            );

            debug("Invoking the getJson method");
            // Call the getJson method
            let json_instance = (self.jni_call_object_method)(
                self.jni_env,
                instance.jinstance,
                get_json_method,
            );
            let _ = self.do_return("")?;
            debug("Transforming jstring to rust String");
            let json = self.to_rust_string(json_instance as jstring)?;
            self.do_return(serde_json::from_str(&json)?)
        }
    }

    /// Deploys a maven artifact in the default j4rs jars location.
    ///
    /// This is useful for build scripts that need jars for the runtime that can be downloaded from Maven.
    ///
    /// The function deploys __only__ the specified artifact, not its transitive dependencies.
    #[deprecated(since = "0.7.0", note = "please use `deploy_artifact` instead")]
    pub fn deploy_maven(&self, artifact: MavenArtifact) -> errors::Result<()> {
        let instance = self.create_instance(
            "org.astonbitecode.j4rs.api.deploy.SimpleMavenDeployer",
            &vec![InvocationArg::from(artifact.base)])?;

        let _ = self.invoke(
            &instance,
            "deploy",
            &vec![
                InvocationArg::from(artifact.group),
                InvocationArg::from(artifact.id),
                InvocationArg::from(artifact.version),
                InvocationArg::from(artifact.qualifier)])?;
        Ok(())
    }

    /// Deploys an artifact in the default j4rs jars location.
    ///
    /// This is useful for build scripts that need jars for the runtime that can be downloaded from e.g. Maven.
    ///
    /// The function deploys __only__ the specified artifact, not its transitive dependencies.
    pub fn deploy_artifact<T: Any + JavaArtifact>(&self, artifact: &T) -> errors::Result<()> {
        let artifact = artifact as &dyn Any;
        if let Some(maven_artifact) = artifact.downcast_ref::<MavenArtifact>() {
            let instance = self.create_instance(
                "org.astonbitecode.j4rs.api.deploy.SimpleMavenDeployer",
                &vec![InvocationArg::from(&maven_artifact.base)])?;

            let _ = self.invoke(
                &instance,
                "deploy",
                &vec![
                    InvocationArg::from(&maven_artifact.group),
                    InvocationArg::from(&maven_artifact.id),
                    InvocationArg::from(&maven_artifact.version),
                    InvocationArg::from(&maven_artifact.qualifier)])?;
            Ok(())
        } else if let Some(local_jar_artifact) = artifact.downcast_ref::<LocalJarArtifact>() {
            let instance = self.create_instance(
                "org.astonbitecode.j4rs.api.deploy.FileSystemDeployer",
                &vec![InvocationArg::from(&local_jar_artifact.base)])?;

            let _ = self.invoke(
                &instance,
                "deploy",
                &vec![InvocationArg::from(&local_jar_artifact.path)])?;
            Ok(())
        } else {
            Err(J4RsError::GeneralError(format!("Don't know how to deploy artifacts of {:?}", artifact.type_id())))
        }
    }

    /// Copies the jassets default directory and the j4rs dynamic library under the specified location.
    /// This is useful for cases when `with_base_path` method is used when building a Jvm with the JvmBuilder.
    /// Build scripts should use this method.
    pub fn copy_j4rs_libs_under(path: &str) -> errors::Result<()> {
        let mut pb = PathBuf::from(path);
        pb.push("deps");
        fs::create_dir_all(&pb)?;

        let default_jassets_path_buf = utils::default_jassets_path()?;
        let default_jassets_path_string = default_jassets_path_buf.to_str().unwrap().to_owned();

        // Copy the jassets
        let ref mut options = fs_extra::dir::CopyOptions::new();
        options.overwrite = true;
        let _ = fs_extra::copy_items(vec![default_jassets_path_string].as_ref(), path, options)?;

        // Copy the dynamic libraries
        let dynlibs: Vec<String> = utils::find_j4rs_dynamic_libraries_paths()?;

        let _ = fs_extra::copy_items(&dynlibs, &pb, options)?;

        Ok(())
    }

    /// Initiates a chain of operations on Instances.
    pub fn chain(&self, instance: Instance) -> ChainableInstance {
        ChainableInstance::new(instance, &self)
    }

    fn do_return<T>(&self, to_return: T) -> errors::Result<T> {
        unsafe {
            if (self.jni_exception_check)(self.jni_env) == JNI_TRUE {
                (self.jni_exception_describe)(self.jni_env);
                (self.jni_exception_clear)(self.jni_env);
                Err(errors::J4RsError::JavaError("An Exception was thrown by Java... Please check the logs or the console.".to_string()))
            } else {
                Ok(to_return)
            }
        }
    }

    // Retrieves a JNIEnv in the case that a JVM is already created even from another thread.
    fn get_created_vm() -> Option<*mut JNIEnv> {
        unsafe {
            // Get the number of the already created VMs. This is most probably 1, but we retrieve the number just in case...
            let mut created_vms_size: jsize = 0;
            tweaks::get_created_java_vms(&mut Vec::new(), 0, &mut created_vms_size);

            if created_vms_size == 0 {
                None
            } else {
                debug(&format!("Retrieving the first of {} created JVMs", created_vms_size));
                // Get the created VM
                let mut buffer: Vec<*mut JavaVM> = Vec::new();
                for _ in 0..created_vms_size { buffer.push(ptr::null_mut()); }

                let retjint = tweaks::get_created_java_vms(&mut buffer, created_vms_size, &mut created_vms_size);
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
                            error("Cannot attach the thread to the JVM");
                            None
                        }
                    }
                } else {
                    error(&format!("Error while retrieving the created JVMs: {}", retjint));
                    None
                }
            }
        }
    }

    fn detach_current_thread(&self) {
        unsafe {
            // Get the number of the already created VMs. This is most probably 1, but we retrieve the number just in case...
            let mut created_vms_size: jsize = 0;
            tweaks::get_created_java_vms(&mut Vec::new(), 0, &mut created_vms_size);

            if created_vms_size > 0 {
                // Get the created VM
                let mut buffer: Vec<*mut JavaVM> = Vec::new();
                for _ in 0..created_vms_size { buffer.push(ptr::null_mut()); }

                let retjint = tweaks::get_created_java_vms(&mut buffer, created_vms_size, &mut created_vms_size);
                if retjint == JNI_OK {
                    match (**buffer[0]).DetachCurrentThread {
                        Some(dct) => {
                            (dct)(buffer[0]);
                        }
                        None => {
                            warn("Cannot detach the thread from the JVM");
                        }
                    }
                } else {
                    warn(&format!("Error while retrieving the created JVMs: {}", retjint));
                }
            }
        }
    }

    pub fn to_rust_string(&self, java_string: jstring) -> errors::Result<String> {
        unsafe {
            let s = (self.jni_get_string_utf_chars)(
                self.jni_env,
                java_string,
                ptr::null_mut(),
            );
            let rust_string = utils::to_rust_string(s);
            self.do_return(rust_string)
        }
    }
}

impl Drop for Jvm {
    fn drop(&mut self) {
        if remove_active_jvm() <= 0 {
            if self.detach_thread_on_drop {
                self.detach_current_thread();
            }
            set_thread_local_env(None);
        }
    }
}

/// A builder for Jvm
pub struct JvmBuilder<'a> {
    classpath_entries: Vec<ClasspathEntry<'a>>,
    java_opts: Vec<JavaOpt<'a>>,
    no_implicit_classpath: bool,
    detach_thread_on_drop: bool,
    lib_name_opt: Option<String>,
    skip_setting_native_lib: bool,
    base_path: Option<String>,
}

impl<'a> JvmBuilder<'a> {
    /// Creates a new JvmBuilder.
    pub fn new<'b>() -> JvmBuilder<'b> {
        JvmBuilder {
            classpath_entries: Vec::new(),
            java_opts: Vec::new(),
            no_implicit_classpath: false,
            detach_thread_on_drop: true,
            lib_name_opt: None,
            skip_setting_native_lib: false,
            base_path: None,
        }
    }

    /// Adds a classpath entry.
    pub fn classpath_entry(&'a mut self, cp_entry: ClasspathEntry<'a>) -> &'a mut JvmBuilder {
        self.classpath_entries.push(cp_entry);
        self
    }

    /// Adds classpath entries.
    pub fn classpath_entries(&'a mut self, cp_entries: Vec<ClasspathEntry<'a>>) -> &'a mut JvmBuilder {
        for cp_entry in cp_entries {
            self.classpath_entries.push(cp_entry);
        }
        self
    }

    /// Adds a Java option.
    pub fn java_opt(&'a mut self, opt: JavaOpt<'a>) -> &'a mut JvmBuilder {
        self.java_opts.push(opt);
        self
    }

    /// Adds Java options.
    pub fn java_opts(&'a mut self, opts: Vec<JavaOpt<'a>>) -> &'a mut JvmBuilder {
        for opt in opts {
            self.java_opts.push(opt);
        }
        self
    }

    /// By default, the created `Jvm`s include an implicit classpath entry that includes the j4rs jar.
    /// When `with_no_implicit_classpath()` is called, this classpath will not be added to the Jvm.
    pub fn with_no_implicit_classpath(&'a mut self) -> &'a mut JvmBuilder {
        self.no_implicit_classpath = true;
        self
    }

    /// When a Jvm goes out of scope and is being dropped, its current thread is being detached from the Java VM.
    /// A Jvm that is created with `detach_thread_on_drop(false)` will not detach the thread when being dropped.
    ///
    /// This is useful when in the Java world a native method is called and in the native code someone needs to create a j4rs Jvm.
    /// If that Jvm detaches its current thread when being dropped, there will be problems for the Java world code to continue executing.
    pub fn detach_thread_on_drop(&'a mut self, detach_thread_on_drop: bool) -> &'a mut JvmBuilder {
        self.detach_thread_on_drop = detach_thread_on_drop;
        self
    }

    /// In the case that the j4rs is statically linked to some other library, the Java world (j4rs.jar) needs to load that
    /// library instead of the default one.
    ///
    /// This function defines the native library name to load.
    pub fn with_native_lib_name(&'a mut self, lib_name: &str) -> &'a mut JvmBuilder {
        self.lib_name_opt = Some(lib_name.to_string());
        self
    }

    /// Instructs the builder not to instruct the Java world j4rs code not to load the native library.
    /// (most probably because it is already loaded)
    pub fn skip_setting_native_lib(&'a mut self) -> &'a mut JvmBuilder {
        self.skip_setting_native_lib = true;
        self
    }

    /// Defines the location of the jassets and deps directory.
    /// The jassets contains the j4rs jar and the deps the j4rs dynamic library.
    pub fn with_base_path(&'a mut self, base_path: &str) -> &'a mut JvmBuilder {
        self.base_path = Some(base_path.to_string());
        self
    }

    /// Creates a Jvm
    pub fn build(&self) -> errors::Result<Jvm> {
        let classpath = if self.no_implicit_classpath {
            self.classpath_entries
                .iter()
                .fold(
                    ".".to_string(),
                    |all, elem| {
                        format!("{}{}{}", all, utils::classpath_sep(), elem.to_string())
                    })
        } else {
            // The default classpath contains all the jars in the jassets directory
            let jassets_path = match &self.base_path {
                Some(base_path_string) => {
                    let mut pb = PathBuf::from(base_path_string);
                    pb.push("jassets");
                    let mut global_jassets_path_opt = JASSETS_PATH.lock()?;
                    *global_jassets_path_opt = Some(pb.clone());
                    pb
                }
                None => utils::default_jassets_path()?,
            };
            let all_jars = get_dir_content(&jassets_path)?.files;
            // This is the j4rs jar that should be included in the classpath
            let j4rs_jar_to_use = format!("j4rs-{}-jar-with-dependencies.jar", j4rs_version());
            // Filter out possible incorrect jars of j4rs
            let filtered_jars: Vec<String> = all_jars.into_iter()
                .filter(|jar| {
                    !jar.contains("j4rs-") || jar.ends_with(&j4rs_jar_to_use)
                })
                .collect();
            let cp_string = filtered_jars.join(utils::classpath_sep());

            let default_class_path = format!("-Djava.class.path={}", cp_string);

            self.classpath_entries
                .iter()
                .fold(
                    default_class_path,
                    |all, elem| {
                        format!("{}{}{}", all, utils::classpath_sep(), elem.to_string())
                    })
        };
        info(&format!("Setting classpath to {}", classpath));

        // Populate the JVM Options
        let mut jvm_options = if self.no_implicit_classpath {
            vec![classpath]
        } else {
            let default_library_path = utils::java_library_path()?;
            info(&format!("Setting library path to {}", default_library_path));
            vec![classpath, default_library_path]
        };
        self.java_opts.clone().into_iter().for_each(|opt| jvm_options.push(opt.to_string()));

        let deps_dir = utils::deps_dir()?;
        // Pass to the Java world the name of the j4rs library.
        let lib_name_opt = if self.lib_name_opt.is_none() && !self.skip_setting_native_lib {
            let found_libs: Vec<String> = if Path::new(&deps_dir).exists() {
                utils::find_j4rs_dynamic_libraries_names()?
            } else {
                // If deps dir is not found, fallback to default naming in order for the library to be searched in the default
                // library locations of the system.
                let default_lib_name = if cfg!(windows) {
                    "l4rs.dll".to_string()
                } else {
                    "libj4rs.so".to_string()
                };
                info(&format!("Deps directory not found. Setting the library name to search to default: {}", default_lib_name));
                vec![default_lib_name]
            };

            let lib_name_opt = if found_libs.len() > 0 {
                let a_lib = found_libs[0].clone().replace("lib", "");

                let dot_splitted: Vec<&str> = a_lib.split(".").collect();
                let name = dot_splitted[0].to_string();
                info(&format!("Passing to the Java world the name of the library to load: {}", name));
                Some(name)
            } else {
                None
            };
            lib_name_opt
        } else if self.lib_name_opt.is_some() && !self.skip_setting_native_lib {
            let name = self.lib_name_opt.clone();
            info(&format!("Passing to the Java world the name of the library to load: {}", name.as_ref().unwrap()));
            name
        } else {
            None
        };

        Jvm::new(&jvm_options, lib_name_opt)
            .and_then(|mut jvm| {
                if !self.detach_thread_on_drop {
                    jvm.detach_thread_on_drop(false);
                }
                Ok(jvm)
            })
    }

    /// Creates a Jvm, similar with an already created j4rs Jvm.
    ///
    /// _Note: The already created Jvm is a j4rs Jvm, not a Java VM._
    pub fn already_initialized() -> errors::Result<Jvm> {
        Jvm::new(&Vec::new(), None)
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

            debug(&format!("Calling the InvocationArg constructor with '{}'", class_name));
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

            debug(&format!("Calling the InvocationArg constructor for class '{}'", class_name));

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

//impl Drop for InvocationArg {
//    fn drop(&mut self) {
////        delete_java_ref(self.jni_env, self.jinstance);
//    }
//}

impl<'a> From<(&'a str, &'a str)> for InvocationArg {
    fn from(tup: (&'a str, &'a str)) -> InvocationArg {
        InvocationArg::Rust {
            json: tup.0.to_string(),
            class_name: tup.1.to_string(),
            arg_from: RUST.to_string(),
        }
    }
}

impl From<Instance> for InvocationArg {
    fn from(instance: Instance) -> InvocationArg {
        let class_name = instance.class_name.to_owned();

        InvocationArg::Java {
            instance: instance,
            class_name: class_name,
            arg_from: JAVA.to_string(),
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
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
        InvocationArg::from(res.unwrap())
    }
}

// TODO: Use try_from when it becomes stable (Use ParseError in case of error)
impl<'a, 'b, T> From<(&'a [T], &'a str, &'b Jvm)> for InvocationArg where T: Serialize {
    fn from(vec_t_tup: (&'a [T], &'a str, &'b Jvm)) -> InvocationArg {
        let (vec, elements_class_name, jvm) = vec_t_tup;
        let args: Vec<InvocationArg> = vec.iter().map(|elem| InvocationArg::new(elem, elements_class_name)).collect();
        let wrapper_arg = InvocationArg::new(&args, J4RS_ARRAY);
        let res = jvm.invoke_static("java.util.Arrays", "asList", vec![wrapper_arg].as_slice());
        InvocationArg::from(res.unwrap())
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

/// A receiver for Java Instances.
///
/// It keeps a channel Receiver to get callback Instances from the Java world
/// and the address of a Box<Sender<Instance>> Box in the heap. This Box is used by Java to communicate
/// asynchronously Instances to Rust.
///
/// On Drop, the InstanceReceiver removes the Box from the heap.
pub struct InstanceReceiver {
    rx: Box<Receiver<Instance>>,
    tx_address: i64,
}

impl InstanceReceiver {
    fn new(rx: Receiver<Instance>, tx_address: i64) -> InstanceReceiver {
        InstanceReceiver {
            rx: Box::new(rx),
            tx_address,
        }
    }

    pub fn rx(&self) -> &Receiver<Instance> {
        &self.rx
    }
}

impl Drop for InstanceReceiver {
    fn drop(&mut self) {
        debug("Dropping an InstanceReceiver");
        let p = self.tx_address as *mut Sender<Instance>;
        unsafe {
            let tx = Box::from_raw(p);
            mem::drop(tx);
        }
    }
}

/// A Java instance
#[derive(Serialize)]
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

    pub fn from(obj: jobject) -> errors::Result<Instance> {
        let _jvm = get_thread_local_env().map_err(|_| {
            Jvm::attach_thread()
        });

        let global = create_global_ref_from_local_ref(obj, get_thread_local_env()?)?;
        Ok(Instance {
            jinstance: global,
            class_name: UNKNOWN_FOR_RUST.to_string(),
        })
    }

    /// Creates a weak reference of this Instance.
    fn _weak_ref(&self) -> errors::Result<Instance> {
        Ok(Instance {
            class_name: self.class_name.clone(),
            jinstance: _create_weak_global_ref_from_global_ref(self.jinstance.clone(), get_thread_local_env()?)?,
        })
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        debug(&format!("Dropping an instance of {}", self.class_name));
        if let Some(j_env) = get_thread_local_env_opt() {
            delete_java_ref(j_env, self.jinstance);
        }
    }
}

unsafe impl Send for Instance {}

/// Allows chained Jvm calls to created Instances
pub struct ChainableInstance<'a> {
    instance: Instance,
    jvm: &'a Jvm,
}

impl<'a> ChainableInstance<'a> {
    fn new(instance: Instance, jvm: &'a Jvm) -> ChainableInstance {
        ChainableInstance { instance, jvm }
    }

    pub fn collect(self) -> Instance {
        self.instance
    }

    /// Invokes the method `method_name` of a this `Instance`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke(&self, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.invoke(&self.instance, method_name, inv_args)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Creates a clone of the Instance
    pub fn clone_instance(&self) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.clone_instance(&self.instance)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn cast(&self, to_class: &str) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.cast(&self.instance, to_class)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Retrieves the field `field_name` of the `Instance`.
    pub fn field(&self, field_name: &str) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.field(&self.instance, field_name)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Returns the Rust representation of the provided instance
    pub fn to_rust<T>(self) -> errors::Result<T> where T: DeserializeOwned {
        self.jvm.to_rust(self.instance)
    }
}

/// Marker trait to be used for deploying artifacts.
pub trait JavaArtifact {}

#[derive(Debug)]
pub struct LocalJarArtifact {
    base: String,
    path: String,
}

impl LocalJarArtifact {
    pub fn new(path: &str) -> LocalJarArtifact {
        LocalJarArtifact {
            base: utils::jassets_path().unwrap_or(PathBuf::new()).to_str().unwrap_or("").to_string(),
            path: path.to_string(),
        }
    }
}

impl JavaArtifact for LocalJarArtifact {}

impl<'a> From<&'a str> for LocalJarArtifact {
    fn from(string: &'a str) -> LocalJarArtifact {
        LocalJarArtifact::new(string)
    }
}

impl From<String> for LocalJarArtifact {
    fn from(string: String) -> LocalJarArtifact {
        LocalJarArtifact::new(&string)
    }
}

#[derive(Debug)]
pub struct MavenArtifact {
    base: String,
    group: String,
    id: String,
    version: String,
    qualifier: String,
}

impl JavaArtifact for MavenArtifact {}

impl From<&[&str]> for MavenArtifact {
    fn from(slice: &[&str]) -> MavenArtifact {
        MavenArtifact {
            base: utils::jassets_path().unwrap_or(PathBuf::new()).to_str().unwrap_or("").to_string(),
            group: slice.get(0).unwrap_or(&"").to_string(),
            id: slice.get(1).unwrap_or(&"").to_string(),
            version: slice.get(2).unwrap_or(&"").to_string(),
            qualifier: slice.get(3).unwrap_or(&"").to_string(),
        }
    }
}

impl From<Vec<&str>> for MavenArtifact {
    fn from(v: Vec<&str>) -> MavenArtifact {
        MavenArtifact::from(v.as_slice())
    }
}

impl From<&Vec<&str>> for MavenArtifact {
    fn from(v: &Vec<&str>) -> MavenArtifact {
        MavenArtifact::from(v.as_slice())
    }
}

impl<'a> From<&'a str> for MavenArtifact {
    fn from(string: &'a str) -> MavenArtifact {
        let v: Vec<&str> = string.split(':').collect();
        MavenArtifact::from(v.as_slice())
    }
}

impl From<String> for MavenArtifact {
    fn from(string: String) -> MavenArtifact {
        let v: Vec<&str> = string.split(':').collect();
        MavenArtifact::from(v.as_slice())
    }
}

pub(crate) fn create_global_ref_from_local_ref(local_ref: jobject, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        match ((**jni_env).NewGlobalRef,
               (**jni_env).DeleteLocalRef,
               (**jni_env).ExceptionCheck,
               (**jni_env).ExceptionDescribe,
               (**jni_env).ExceptionClear,
               (**jni_env).GetObjectRefType) {
            (Some(ngr), Some(dlr), Some(exc), Some(exd), Some(exclear), Some(gort)) => {
                // Create the global ref
                let global = ngr(
                    jni_env,
                    local_ref,
                );
                // If local ref, delete it
                if gort(jni_env, local_ref) as jint == jobjectRefType::JNILocalRefType as jint {
                    dlr(
                        jni_env,
                        local_ref,
                    );
                }
                // Exception check
                if (exc)(jni_env) == JNI_TRUE {
                    (exd)(jni_env);
                    (exclear)(jni_env);
                    Err(errors::J4RsError::JavaError("An Exception was thrown by Java while creating global ref... Please check the logs or the console.".to_string()))
                } else {
                    Ok(global)
                }
            }
            (_, _, _, _, _, _) => {
                Err(errors::J4RsError::JavaError("Could retrieve the native functions to create a global ref. This may lead to memory leaks".to_string()))
            }
        }
    }
}

fn _create_weak_global_ref_from_global_ref(global_ref: jobject, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        match ((**jni_env).NewWeakGlobalRef,
               (**jni_env).ExceptionCheck,
               (**jni_env).ExceptionDescribe,
               (**jni_env).ExceptionClear) {
            (Some(nwgr), Some(exc), Some(exd), Some(exclear)) => {
                // Create the weak global ref
                let global = nwgr(
                    jni_env,
                    global_ref,
                );
                // Exception check
                if (exc)(jni_env) == JNI_TRUE {
                    (exd)(jni_env);
                    (exclear)(jni_env);
                    Err(errors::J4RsError::JavaError("An Exception was thrown by Java while creating a weak global ref... Please check the logs or the console.".to_string()))
                } else {
                    Ok(global)
                }
            }
            (_, _, _, _) => {
                Err(errors::J4RsError::JavaError("Could retrieve the native functions to create a weak global ref.".to_string()))
            }
        }
    }
}

/// Deletes the java ref from the memory
fn delete_java_ref(jni_env: *mut JNIEnv, jinstance: jobject) {
    unsafe {
        match ((**jni_env).DeleteGlobalRef,
               (**jni_env).ExceptionCheck,
               (**jni_env).ExceptionDescribe,
               (**jni_env).ExceptionClear) {
            (Some(dlr), Some(exc), Some(exd), Some(exclear)) => {
                dlr(
                    jni_env,
                    jinstance,
                );
                if (exc)(jni_env) == JNI_TRUE {
                    (exd)(jni_env);
                    (exclear)(jni_env);
                    error("An Exception was thrown by Java... Please check the logs or the console.");
                }
            }
            (_, _, _, _) => {
                error("Could retrieve the native functions to drop the Java ref. This may lead to memory leaks");
            }
        }
    }
}

/// Deletes the java ref from the memory
fn delete_java_local_ref(jni_env: *mut JNIEnv, jinstance: jobject) {
    unsafe {
        match ((**jni_env).DeleteLocalRef,
               (**jni_env).ExceptionCheck,
               (**jni_env).ExceptionDescribe,
               (**jni_env).ExceptionClear) {
            (Some(dlr), Some(exc), Some(exd), Some(exclear)) => {
                dlr(
                    jni_env,
                    jinstance,
                );
                if (exc)(jni_env) == JNI_TRUE {
                    (exd)(jni_env);
                    (exclear)(jni_env);
                    error("An Exception was thrown by Java... Please check the logs or the console.");
                }
            }
            (_, _, _, _) => {
                error("Could retrieve the native functions to drop the Java ref. This may lead to memory leaks");
            }
        }
    }
}

/// A classpath entry.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
    use serde_json;

    use super::*;

    #[test]
    fn jvm_builder() {
        let res = JvmBuilder::new().build();
        assert!(res.is_ok());
        let one_more_res = JvmBuilder::already_initialized();
        assert!(one_more_res.is_ok());
    }

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
    fn invocation_arg_from_primitive_types() {
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

    #[test]
    fn maven_artifact_from() {
        let ma1 = MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1");
        assert_eq!(ma1.group, "io.github.astonbitecode");
        assert_eq!(ma1.id, "j4rs");
        assert_eq!(ma1.version, "0.5.1");
        assert_eq!(ma1.qualifier, "");

        let ma2 = MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1".to_string());
        assert_eq!(ma2.group, "io.github.astonbitecode");
        assert_eq!(ma2.id, "j4rs");
        assert_eq!(ma2.version, "0.5.1");
        assert_eq!(ma2.qualifier, "");

        let ma3 = MavenArtifact::from(&vec!["io.github.astonbitecode", "j4rs", "0.5.1"]);
        assert_eq!(ma3.group, "io.github.astonbitecode");
        assert_eq!(ma3.id, "j4rs");
        assert_eq!(ma3.version, "0.5.1");
        assert_eq!(ma3.qualifier, "");
    }

    #[test]
    fn test_copy_j4rs_libs_under() {
        let newdir = "./newdir";
        Jvm::copy_j4rs_libs_under(newdir).unwrap();

        let _ = fs_extra::remove_items(&vec![newdir]);
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