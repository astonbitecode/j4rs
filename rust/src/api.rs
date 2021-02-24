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

use std::{fs, mem, thread, time};
use std::any::Any;
use std::convert::TryFrom;
use std::env;
use std::ops::Drop;
use std::os::raw::c_void;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::ptr;
use std::sync::mpsc::{channel, Receiver, Sender};

use fs_extra::dir::get_dir_content;
use jni_sys::{
    self,
    JavaVM,
    JavaVMInitArgs,
    JavaVMOption,
    jint,
    JNI_EDETACHED,
    JNI_EEXIST,
    JNI_EINVAL,
    JNI_ENOMEM,
    JNI_ERR,
    JNI_EVERSION,
    JNI_OK,
    JNI_TRUE,
    JNI_VERSION_1_8,
    JNIEnv,
    jobject,
    jsize,
    jstring,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

use crate::{api_tweaks as tweaks, cache, MavenSettings};
use crate::errors;
use crate::errors::{J4RsError, opt_to_res};
use crate::jni_utils;
use crate::provisioning::{get_maven_settings, JavaArtifact, LocalJarArtifact, MavenArtifact};
use crate::provisioning;
use crate::utils;

use super::logger::{debug, error, info, warn};

// Initialize the environment
include!(concat!(env!("OUT_DIR"), "/j4rs_init.rs"));

pub(crate) const CLASS_OBJECT: &'static str = "java.lang.Object";
pub(crate) const CLASS_STRING: &'static str = "java.lang.String";
pub(crate) const CLASS_BOOLEAN: &'static str = "java.lang.Boolean";
pub(crate) const CLASS_BYTE: &'static str = "java.lang.Byte";
pub(crate) const CLASS_CHARACTER: &'static str = "java.lang.Character";
pub(crate) const CLASS_SHORT: &'static str = "java.lang.Short";
pub(crate) const CLASS_INTEGER: &'static str = "java.lang.Integer";
pub(crate) const CLASS_LONG: &'static str = "java.lang.Long";
pub(crate) const CLASS_FLOAT: &'static str = "java.lang.Float";
pub(crate) const CLASS_DOUBLE: &'static str = "java.lang.Double";
pub(crate) const CLASS_LIST: &'static str = "java.util.List";
pub(crate) const CLASS_NATIVE_CALLBACK_TO_RUST_CHANNEL_SUPPORT: &'static str = "org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport";
pub(crate) const CLASS_J4RS_EVENT_HANDLER: &'static str = "org.astonbitecode.j4rs.api.jfx.handlers.J4rsEventHandler";
pub(crate) const CLASS_J4RS_FXML_LOADER: &'static str = "org.astonbitecode.j4rs.api.jfx.J4rsFxmlLoader";
pub const _JNI_VERSION_10: jint = 0x000a0000;

pub type Callback = fn(Jvm, Instance) -> ();

/// Holds the assets for the JVM
#[derive(Clone)]
pub struct Jvm {
    pub(crate) jni_env: *mut JNIEnv,
    detach_thread_on_drop: bool,
}

impl Jvm {
    /// Creates a new Jvm.
    pub fn new(jvm_options: &[String], lib_name_to_load: Option<String>) -> errors::Result<Jvm> {
        Self::create_jvm(jvm_options, lib_name_to_load)
    }

    /// Attaches the current thread to an active JavaVM
    pub fn attach_thread() -> errors::Result<Jvm> {
        Self::create_jvm(&[], None)
    }

    /// Attaches the current thread to an active JavaVM and instructs that the Jvm will detach the Java JVM
    /// from the thread when the rust Jvm is dropped.
    ///
    /// This is useful when creating a Jvm while on a Thread that is created in the Java world.
    /// When this Jvm is dropped, we don't want to detach the thread from the Java VM.
    pub fn attach_thread_with_no_detach_on_drop() -> errors::Result<Jvm> {
        let mut jvm = Jvm::attach_thread()?;
        jvm.detach_thread_on_drop(false);
        Ok(jvm)
    }

    /// If false, the thread will not be detached when the Jvm is being dropped.
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
        let _g = cache::MUTEX.lock()?;

        let result = if let Some(env) = cache::get_thread_local_env_opt() {
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
                        let cstr = utils::to_c_string(opt);
                        let jo = JavaVMOption {
                            optionString: utils::to_c_string(opt),
                            extraInfo: ptr::null_mut() as *mut c_void,
                        };
                        utils::drop_c_string(cstr);
                        jo
                    })
                    .collect();

                let mut jvm_arguments = JavaVMInitArgs {
                    version: JNI_VERSION_1_8,
                    nOptions: jvm_options.len() as i32,
                    options: jvm_options_vec.as_mut_ptr(),
                    ignoreUnrecognized: JNI_TRUE,
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
                jvm.invoke_static(CLASS_NATIVE_CALLBACK_TO_RUST_CHANNEL_SUPPORT,
                                  "initialize",
                                  &vec![InvocationArg::try_from(libname)?])?;
                debug("NativeCallbackSupport initialized");
            }

            Ok(jvm)
        }
    }

    pub fn try_from(jni_environment: *mut JNIEnv) -> errors::Result<Jvm> {
        if cache::get_thread_local_env_opt().is_none() {
            // Create and set the environment in Thread Local
            unsafe {
                let _ = cache::get_jni_get_method_id().or_else(|| { cache::set_jni_get_method_id((**jni_environment).GetMethodID) });
                let _ = cache::get_jni_get_static_method_id().or_else(|| cache::set_jni_get_static_method_id((**jni_environment).GetStaticMethodID));
                let _ = cache::get_jni_new_object().or_else(|| cache::set_jni_new_object((**jni_environment).NewObject));
                let _ = cache::get_jni_new_string_utf().or_else(|| cache::set_jni_new_string_utf((**jni_environment).NewStringUTF));
                let _ = cache::get_jni_get_string_utf_chars().or_else(|| cache::set_jni_get_string_utf_chars((**jni_environment).GetStringUTFChars));
                let _ = cache::get_jni_release_string_utf_chars().or_else(|| cache::set_jni_release_string_utf_chars((**jni_environment).ReleaseStringUTFChars));
                let _ = cache::get_jni_call_object_method().or_else(|| cache::set_jni_call_object_method((**jni_environment).CallObjectMethod));
                let _ = cache::get_jni_call_float_method().or_else(|| cache::set_jni_call_float_method((**jni_environment).CallFloatMethod));
                let _ = cache::get_jni_call_double_method().or_else(|| cache::set_jni_call_double_method((**jni_environment).CallDoubleMethod));
                let _ = cache::get_jni_call_void_method().or_else(|| cache::set_jni_call_void_method((**jni_environment).CallVoidMethod));
                let _ = cache::get_jni_call_static_object_method().or_else(|| cache::set_jni_call_static_object_method((**jni_environment).CallStaticObjectMethod));
                let _ = cache::get_jni_new_object_array().or_else(|| cache::set_jni_new_object_array((**jni_environment).NewObjectArray));
                let _ = cache::get_jni_set_object_array_element().or_else(|| cache::set_jni_set_object_array_element((**jni_environment).SetObjectArrayElement));
                let ec = cache::get_jni_exception_check().or_else(|| cache::set_jni_exception_check((**jni_environment).ExceptionCheck));
                let ed = cache::get_jni_exception_describe().or_else(|| cache::set_jni_exception_describe((**jni_environment).ExceptionDescribe));
                let exclear = cache::get_jni_exception_clear().or_else(|| cache::set_jni_exception_clear((**jni_environment).ExceptionClear));
                let _ = cache::get_jni_delete_local_ref().or_else(|| cache::set_jni_delete_local_ref((**jni_environment).DeleteLocalRef));
                let _ = cache::get_jni_delete_global_ref().or_else(|| cache::set_jni_delete_global_ref((**jni_environment).DeleteGlobalRef));
                let _ = cache::get_jni_new_global_ref().or_else(|| cache::set_jni_new_global_ref((**jni_environment).NewGlobalRef));
                let _ = cache::get_jni_throw_new().or_else(|| cache::set_jni_throw_new((**jni_environment).ThrowNew));
                let _ = cache::get_is_same_object().or_else(|| cache::set_is_same_object((**jni_environment).IsSameObject));

                match (ec, ed, exclear) {
                    (Some(ec), Some(ed), Some(exclear)) => {
                        if (ec)(jni_environment) == JNI_TRUE {
                            (ed)(jni_environment);
                            (exclear)(jni_environment);
                            Err(errors::J4RsError::JavaError("The VM cannot be started... Please check the logs.".to_string()))
                        } else {
                            let jvm = Jvm {
                                jni_env: jni_environment,
                                detach_thread_on_drop: true,
                            };

                            cache::set_thread_local_env(Some(jni_environment));
                            cache::add_active_jvm();

                            Ok(jvm)
                        }
                    }
                    (_, _, _) => {
                        Err(errors::J4RsError::JniError(format!("Could not initialize the JVM: Error while trying to retrieve JNI functions.")))
                    }
                }
            }
        } else {
            // Use the environment from the Thread Local
            let jvm = Jvm {
                jni_env: jni_environment,
                detach_thread_on_drop: true,
            };

            cache::set_thread_local_env(Some(jni_environment));
            cache::add_active_jvm();

            Ok(jvm)
        }
    }

    /// Creates an `Instance` of the class `class_name`, passing an array of `InvocationArg`s to construct the instance.
    pub fn create_instance(&self, class_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Instantiating class {} using {} arguments", class_name, inv_args.len()));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;

            // Factory invocation - rest of the arguments: Create a new objectarray of class InvocationArg
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

            // Factory invocation - rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java = inv_args[i as usize].as_java_ptr_with_global_ref(self.jni_env)?;
                // Set it in the array
                (opt_to_res(cache::get_jni_set_object_array_element())?)(
                    self.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
                inv_arg_jobjects.push(inv_arg_java);
            }
            // Call the method of the factory that instantiates a new class of `class_name`.
            // This returns a Instance that acts like a proxy to the Java world.
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                self.jni_env,
                cache::get_factory_class()?,
                cache::get_factory_instantiate_method()?,
                class_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(self.jni_env, ())?;

            let java_instance_global_instance = jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            jni_utils::delete_java_ref(self.jni_env, array_ptr);
            jni_utils::delete_java_ref(self.jni_env, class_name_jstring);
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(self.jni_env, inv_arg_jobject);
            }

            // Create and return the Instance
            Self::do_return(self.jni_env, Instance {
                jinstance: java_instance_global_instance,
                class_name: class_name.to_string(),
                skip_deleting_jobject: false,
            })
        }
    }

    /// Retrieves the static class `class_name`.
    pub fn static_class(&self, class_name: &str) -> errors::Result<Instance> {
        debug(&format!("Retrieving static class {}", class_name));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;

            // Call the method of the factory that creates a Instance for static calls to methods of class `class_name`.
            // This returns a Instance that acts like a proxy to the Java world.
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                self.jni_env,
                cache::get_factory_class()?,
                cache::get_factory_create_for_static_method()?,
                class_name_jstring,
            );

            jni_utils::delete_java_ref(self.jni_env, class_name_jstring);

            // Create and return the Instance.
            Self::do_return(self.jni_env, Instance::from_jobject_with_global_ref(java_instance)?)
        }
    }

    /// Creates a new Java Array with elements of the class `class_name`.
    /// The array will have the `InvocationArg`s populated.
    /// The `InvocationArg`s __must__ be of type _class_name_.
    pub fn create_java_array(&self, class_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Creating a java array of class {} with {} elements", class_name, inv_args.len()));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;

            // Factory invocation - rest of the arguments: Create a new objectarray of class InvocationArg
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

            // Factory invocation - rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java = inv_args[i as usize].as_java_ptr_with_global_ref(self.jni_env)?;
                // Set it in the array
                (opt_to_res(cache::get_jni_set_object_array_element())?)(
                    self.jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
                inv_arg_jobjects.push(inv_arg_java);
            }
            // Call the method of the factory that instantiates a new Java Array of `class_name`.
            // This returns a Instance that acts like a proxy to the Java world.
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                self.jni_env,
                cache::get_factory_class()?,
                cache::get_factory_create_java_array_method()?,
                class_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(self.jni_env, ())?;

            let java_instance_global_instance = jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(self.jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(self.jni_env, array_ptr);
            jni_utils::delete_java_ref(self.jni_env, class_name_jstring);

            // Create and return the Instance
            Self::do_return(self.jni_env, Instance {
                jinstance: java_instance_global_instance,
                class_name: class_name.to_string(),
                skip_deleting_jobject: false,
            })
        }
    }

    /// Creates a new Java List with elements of the class `class_name`.
    /// The array will have the `InvocationArg`s populated.
    /// The `InvocationArg`s __must__ be of type _class_name_.
    pub fn create_java_list(&self, class_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        Jvm::do_create_java_list(self.jni_env, class_name, inv_args)
    }

    fn do_create_java_list(jni_env: *mut JNIEnv, class_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Creating a java list of class {} with {} elements", class_name, inv_args.len()));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = jni_utils::global_jobject_from_str(&class_name, jni_env)?;

            // Factory invocation - rest of the arguments: Create a new object list of class InvocationArg
            let size = inv_args.len() as i32;
            let array_ptr = {
                let j = (opt_to_res(cache::get_jni_new_object_array())?)(
                    jni_env,
                    size,
                    cache::get_invocation_arg_class()?,
                    ptr::null_mut(),
                );
                jni_utils::create_global_ref_from_local_ref(j, jni_env)?
            };
            let mut inv_arg_jobjects: Vec<jobject> = Vec::with_capacity(size as usize);

            // Factory invocation - rest of the arguments: populate the array
            for i in 0..size {
                // Create an InvocationArg Java Object
                let inv_arg_java = inv_args[i as usize].as_java_ptr_with_global_ref(jni_env)?;
                // Set it in the array
                (opt_to_res(cache::get_jni_set_object_array_element())?)(
                    jni_env,
                    array_ptr,
                    i,
                    inv_arg_java,
                );
                inv_arg_jobjects.push(inv_arg_java);
            }
            // Call the method of the factory that instantiates a new Java Array of `class_name`.
            // This returns a Instance that acts like a proxy to the Java world.
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                jni_env,
                cache::get_factory_class()?,
                cache::get_factory_create_java_list_method()?,
                class_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(jni_env, ())?;

            let java_instance_global_instance = jni_utils::create_global_ref_from_local_ref(java_instance, jni_env)?;
            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(jni_env, array_ptr);
            jni_utils::delete_java_ref(jni_env, class_name_jstring);

            // Create and return the Instance
            Self::do_return(jni_env, Instance {
                jinstance: java_instance_global_instance,
                class_name: class_name.to_string(),
                skip_deleting_jobject: false,
            })
        }
    }

    /// Invokes the method `method_name` of a created `Instance`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke(&self, instance: &Instance, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Invoking method {} of class {} using {} arguments", method_name, instance.class_name, inv_args.len()));
        unsafe {
            // First argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring = jni_utils::global_jobject_from_str(&method_name, self.jni_env)?;

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
                let inv_arg_java = inv_args[i as usize].as_java_ptr_with_global_ref(self.jni_env)?;
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
            let java_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_invoke_method()?,
                method_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(self.jni_env, ())?;

            let java_instance_global_instance = jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(self.jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(self.jni_env, array_ptr);
            jni_utils::delete_java_ref(self.jni_env, method_name_jstring);

            // Create and return the Instance
            Self::do_return(self.jni_env, Instance {
                jinstance: java_instance_global_instance,
                class_name: cache::UNKNOWN_FOR_RUST.to_string(),
                skip_deleting_jobject: false,
            })
        }
    }

    /// Retrieves the field `field_name` of a created `Instance`.
    pub fn field(&self, instance: &Instance, field_name: &str) -> errors::Result<Instance> {
        debug(&format!("Retrieving field {} of class {}", field_name, instance.class_name));
        unsafe {
            // First argument: create a jstring to pass as argument for the field_name
            let field_name_jstring: jstring = jni_utils::global_jobject_from_str(&field_name, self.jni_env)?;

            // Call the method of the instance
            let java_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_field_method()?,
                field_name_jstring,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(self.jni_env, ())?;

            let java_instance_global_instance = jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            jni_utils::delete_java_ref(self.jni_env, field_name_jstring);

            // Create and return the Instance
            Self::do_return(self.jni_env, Instance {
                jinstance: java_instance_global_instance,
                class_name: cache::UNKNOWN_FOR_RUST.to_string(),
                skip_deleting_jobject: false,
            })
        }
    }
    /// Retrieves the field `field_name` of a static class.
    pub fn static_class_field(&self, class_name: &str, field_name: &str) -> errors::Result<Instance> {
        debug(&format!("Retrieving field {} of static class {}", field_name, class_name));
        let i = self.static_class(class_name)?;
        self.field(&i, &field_name)
    }

    /// Invokes the method `method_name` of a created `Instance`, passing an array of `InvocationArg`s.
    /// It returns a Result of `InstanceReceiver` that may be used to get an underlying `Receiver<Instance>`. The result of the invocation will come via this Receiver.
    pub fn invoke_to_channel(&self, instance: &Instance, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<InstanceReceiver> {
        debug(&format!("Invoking method {} of class {} using {} arguments. The result of the invocation will come via an InstanceReceiver", method_name, instance.class_name, inv_args.len()));
        unsafe {
            // Create the channel
            let (sender, rx) = channel();
            let tx = Box::new(sender);
            // First argument: the address of the channel Sender
            let raw_ptr = Box::into_raw(tx);
            // Find the address of tx
            let address_string = format!("{:p}", raw_ptr);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();

            // Second argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring = jni_utils::global_jobject_from_str(&method_name, self.jni_env)?;

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
                let inv_arg_java = inv_args[i as usize].as_java_ptr_with_global_ref(self.jni_env)?;
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
                cache::get_invoke_to_channel_method()?,
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
            Self::do_return(self.jni_env, InstanceReceiver::new(rx, address))
        }
    }

    /// Initializes a callback channel via a Java Instance that is a `NativeCallbackToRustChannelSupport`.
    /// It returns a Result of `InstanceReceiver` that may be used to get an underlying `Receiver<Instance>`.
    /// The `NativeCallbackToRustChannelSupport` Instance which is passed as argument, will be sending `Instance`s via this Receiver.
    pub fn init_callback_channel(&self, instance: &Instance) -> errors::Result<InstanceReceiver> {
        debug(&format!("Initializing callback channel"));
        unsafe {
            // Create the channel
            let (sender, rx) = channel();
            let tx = Box::new(sender);
            // First argument: the address of the channel Sender
            let raw_ptr = Box::into_raw(tx);
            // Find the address of tx
            let address_string = format!("{:p}", raw_ptr);
            let address = i64::from_str_radix(&address_string[2..], 16).unwrap();

            // Call the method of the instance
            let _ = (opt_to_res(cache::get_jni_call_void_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_init_callback_channel_method()?,
                address,
            );

            // Create and return the Instance
            Self::do_return(self.jni_env, InstanceReceiver::new(rx, address))
        }
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke_static(&self, class_name: &str, method_name: &str, inv_args: &[InvocationArg]) -> errors::Result<Instance> {
        debug(&format!("Invoking static method {} of class {} using {} arguments", method_name, class_name, inv_args.len()));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring = jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;
            // Call the method of the factory that creates a Instance for static calls to methods of class `class_name`.
            // This returns a Instance that acts like a proxy to the Java world.
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                self.jni_env,
                cache::get_factory_class()?,
                cache::get_factory_create_for_static_method()?,
                class_name_jstring,
            );

            // First argument: create a jstring to pass as argument for the method_name
            let method_name_jstring: jstring = jni_utils::global_jobject_from_str(&method_name, self.jni_env)?;

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
                let inv_arg_java = inv_args[i as usize].as_java_ptr_with_global_ref(self.jni_env)?;
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
            let java_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                self.jni_env,
                java_instance,
                cache::get_invoke_static_method()?,
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

            // Create and return the Instance.
            Self::do_return(self.jni_env, Instance::from_jobject_with_global_ref(java_instance)?)
        }
    }

    /// Creates a clone of the provided Instance
    pub fn clone_instance(&self, instance: &Instance) -> errors::Result<Instance> {
        unsafe {
            // Call the clone method
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                self.jni_env,
                cache::get_class_to_invoke_clone_and_cast()?,
                cache::get_clone_static_method()?,
                instance.jinstance,
            );

            // Create and return the Instance
            Self::do_return(self.jni_env, Instance::from_jobject_with_global_ref(java_instance)?)
        }
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn cast(&self, from_instance: &Instance, to_class: &str) -> errors::Result<Instance> {
        debug(&format!("Casting to class {}", to_class));
        unsafe {
            // First argument is the jobject that is inside the from_instance
            // Second argument: create a jstring to pass as argument for the to_class
            let to_class_jstring: jstring = jni_utils::global_jobject_from_str(&to_class, self.jni_env)?;

            // Call the cast method
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                self.jni_env,
                cache::get_class_to_invoke_clone_and_cast()?,
                cache::get_cast_static_method()?,
                from_instance.jinstance,
                to_class_jstring,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(self.jni_env, ())?;

            // Prevent memory leaks from the created local references
            jni_utils::delete_java_ref(self.jni_env, to_class_jstring);

            // Create and return the Instance
            Self::do_return(self.jni_env, Instance::from_jobject_with_global_ref(java_instance)?)
        }
    }

    /// Returns the Rust representation of the provided instance, boxed
    pub fn to_rust_boxed<T>(&self, instance: Instance) -> errors::Result<Box<T>>
        where T: DeserializeOwned + Any {

        // Define the macro inside the function in order to have access to &self
        macro_rules! rust_box_from_java_object {
            ($jni_transformation:path) => {
                {
                    // Call the getObjectMethod. This returns a localref
                    let object_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                        self.jni_env,
                        instance.jinstance,
                        cache::get_get_object_method()?,
                    );
                    let object_instance = jni_utils::create_global_ref_from_local_ref(object_instance, self.jni_env)?;
                    let v = Box::new($jni_transformation(object_instance, self.jni_env)?);
                    let v_any = v as Box<dyn Any>;

                    jni_utils::delete_java_ref(self.jni_env, object_instance);

                    match v_any.downcast::<T>() {
                        Ok(v) => Ok(v),
                        Err(error) => Err(errors::J4RsError::RustError(
                            format!("Could not downcast to Rust type: {:?}", error))),
                    }
                }
            };
        }

        unsafe {
            // Call the getClassName method. This returns a localref
            let object_class_name_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_get_object_class_name_method()?,
            );
            let object_class_name_instance = jni_utils::create_global_ref_from_local_ref(object_class_name_instance, self.jni_env)?;
            let class_name = jni_utils::string_from_jobject(object_class_name_instance, self.jni_env)?;
            jni_utils::delete_java_ref(self.jni_env, object_class_name_instance);
            let to_ret = match class_name.as_str() {
                CLASS_STRING => rust_box_from_java_object!(jni_utils::string_from_jobject),
                CLASS_INTEGER => rust_box_from_java_object!(jni_utils::i32_from_jobject),
                CLASS_BYTE => rust_box_from_java_object!(jni_utils::i8_from_jobject),
                CLASS_SHORT => rust_box_from_java_object!(jni_utils::i16_from_jobject),
                CLASS_LONG => rust_box_from_java_object!(jni_utils::i64_from_jobject),
                CLASS_FLOAT => rust_box_from_java_object!(jni_utils::f32_from_jobject),
                CLASS_DOUBLE => rust_box_from_java_object!(jni_utils::f64_from_jobject),
                CLASS_OBJECT => Ok(Box::new(self.to_rust_deserialized(instance)?)),
                _ => {
                    // Call the getObjectClass method. This returns a localref
                    let object_class_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                        self.jni_env,
                        instance.jinstance,
                        cache::get_get_object_class_method()?,
                    );
                    let object_class_instance = jni_utils::create_global_ref_from_local_ref(object_class_instance, self.jni_env)?;

                    let to_ret_inner = if jni_utils::is_same_object(object_class_instance, cache::get_string_class()?, self.jni_env)? {
                        rust_box_from_java_object!(jni_utils::string_from_jobject)
                    } else if jni_utils::is_same_object(object_class_instance, cache::get_integer_class()?, self.jni_env)? {
                        rust_box_from_java_object!(jni_utils::i32_from_jobject)
                    } else if jni_utils::is_same_object(object_class_instance, cache::get_byte_class()?, self.jni_env)? {
                        rust_box_from_java_object!(jni_utils::i8_from_jobject)
                    } else if jni_utils::is_same_object(object_class_instance, cache::get_short_class()?, self.jni_env)? {
                        rust_box_from_java_object!(jni_utils::i16_from_jobject)
                    } else if jni_utils::is_same_object(object_class_instance, cache::get_long_class()?, self.jni_env)? {
                        rust_box_from_java_object!(jni_utils::i64_from_jobject)
                    } else if jni_utils::is_same_object(object_class_instance, cache::get_float_class()?, self.jni_env)? {
                        rust_box_from_java_object!(jni_utils::f32_from_jobject)
                    } else if jni_utils::is_same_object(object_class_instance, cache::get_double_class()?, self.jni_env)? {
                        rust_box_from_java_object!(jni_utils::f64_from_jobject)
                    } else {
                        Ok(Box::new(self.to_rust_deserialized(instance)?))
                    };
                    jni_utils::delete_java_ref(self.jni_env, object_class_instance);
                    to_ret_inner
                }
            };

            to_ret
        }
    }

    /// Returns the Rust representation of the provided instance
    pub fn to_rust<T>(&self, instance: Instance) -> errors::Result<T> where T: DeserializeOwned + Any {
        self.to_rust_boxed(instance).map(|v| *v)
    }

    pub fn to_rust_deserialized<T>(&self, instance: Instance) -> errors::Result<T> where T: DeserializeOwned + Any {
        unsafe {
            debug("Invoking the getJson method");
            // Call the getJson method. This returns a localref
            let json_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_get_json_method()?,
            );
            let _ = Self::do_return(self.jni_env, "")?;
            debug("Transforming jstring to rust String");
            let global_json_instance = jni_utils::create_global_ref_from_local_ref(json_instance, self.jni_env)?;
            let json = jni_utils::jstring_to_rust_string(&self, global_json_instance as jstring)?;
            jni_utils::delete_java_ref(self.jni_env, global_json_instance);
            Self::do_return(self.jni_env, serde_json::from_str(&json)?)
        }
    }

    /// Deploys an artifact in the default j4rs jars location.
    ///
    /// This is useful for build scripts that need jars for the runtime that can be downloaded from e.g. Maven.
    ///
    /// The function deploys __only__ the specified artifact, not its transitive dependencies.
    pub fn deploy_artifact<T: Any + JavaArtifact>(&self, artifact: &T) -> errors::Result<()> {
        let artifact = artifact as &dyn Any;
        if let Some(maven_artifact) = artifact.downcast_ref::<MavenArtifact>() {
            for repo in get_maven_settings().repos.into_iter() {
                let instance = self.create_instance(
                    "org.astonbitecode.j4rs.api.deploy.SimpleMavenDeployer",
                    &vec![
                        InvocationArg::try_from(repo.uri)?,
                        InvocationArg::try_from(&maven_artifact.base)?])?;

                let res = self.invoke(
                    &instance,
                    "deploy",
                    &vec![
                        InvocationArg::try_from(&maven_artifact.group)?,
                        InvocationArg::try_from(&maven_artifact.id)?,
                        InvocationArg::try_from(&maven_artifact.version)?,
                        InvocationArg::try_from(&maven_artifact.qualifier)?]);

                if res.is_ok() {
                    break;
                }
            }

            Ok(())
        } else if let Some(local_jar_artifact) = artifact.downcast_ref::<LocalJarArtifact>() {
            let instance = self.create_instance(
                "org.astonbitecode.j4rs.api.deploy.FileSystemDeployer",
                &vec![InvocationArg::try_from(&local_jar_artifact.base)?])?;

            let _ = self.invoke(
                &instance,
                "deploy",
                &vec![InvocationArg::try_from(&local_jar_artifact.path)?])?;
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
    pub fn chain(&self, instance: &Instance) -> errors::Result<ChainableInstance> {
        ChainableInstance::new_with_instance_ref(&instance, &self)
    }

    /// Initiates a chain of operations on Instances.
    pub fn into_chain(&self, instance: Instance) -> ChainableInstance {
        ChainableInstance::new(instance, &self)
    }

    /// Throws an exception in the Java World
    pub fn throw_invocation_exception(&self, message: &str) -> errors::Result<()> {
        let _ = jni_utils::throw_exception(message, self.jni_env)?;
        Ok(())
    }

    pub(crate) fn do_return<T>(jni_env: *mut JNIEnv, to_return: T) -> errors::Result<T> {
        unsafe {
            if (opt_to_res(cache::get_jni_exception_check())?)(jni_env) == JNI_TRUE {
                (opt_to_res(cache::get_jni_exception_describe())?)(jni_env);
                (opt_to_res(cache::get_jni_exception_clear())?)(jni_env);
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
            tweaks::get_created_java_vms(&mut Vec::with_capacity(created_vms_size as usize), 0, &mut created_vms_size);

            if created_vms_size == 0 {
                None
            } else {
                debug(&format!("Retrieving the first of {} created JVMs", created_vms_size));
                // Get the created VM (use 2 just in case... :) )
                let mut buffer: Vec<*mut JavaVM> = Vec::with_capacity(2);
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
            tweaks::get_created_java_vms(&mut Vec::with_capacity(created_vms_size as usize), 0, &mut created_vms_size);

            if created_vms_size > 0 {
                // Get the created VM
                let mut buffer: Vec<*mut JavaVM> = Vec::with_capacity(created_vms_size as usize);
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

    /// Returns the first `Instance` that is available from the passed `InstanceReceiver`s,
    /// along with the index of the receiver that was selected and actually returned the instance.
    ///
    /// This is a mostly naive implementation of select, because of [absence for selecting among mpsc channels](https://github.com/rust-lang/rust/issues/27800).
    pub fn select(instance_receivers: &[&InstanceReceiver]) -> errors::Result<(usize, Instance)> {
        loop {
            for (index, ir) in instance_receivers.iter().enumerate() {
                let res = ir.rx.try_recv();
                if res.is_ok() {
                    return Ok((index, res.unwrap()));
                }
            }
            thread::yield_now();
        }
    }

    /// Returns the first `Instance` that is available from the passed `InstanceReceiver`s,
    /// along with the index of the receiver that was selected and actually returned the instance.
    ///
    /// If there are no instances returned for the duration defined in timeout argument, an error is returned.
    ///
    /// This is a mostly naive implementation of select, because of [absence for selecting among mpsc channels](https://github.com/rust-lang/rust/issues/27800).
    pub fn select_timeout(instance_receivers: &[&InstanceReceiver], timeout: &time::Duration) -> errors::Result<(usize, Instance)> {
        let start = time::Instant::now();
        loop {
            for (index, ir) in instance_receivers.iter().enumerate() {
                let res = ir.rx.try_recv();
                if res.is_ok() {
                    return Ok((index, res.unwrap()));
                }
            }
            if &start.elapsed() > timeout {
                return Err(errors::J4RsError::Timeout);
            }
            thread::yield_now();
        }
    }
}

impl Drop for Jvm {
    fn drop(&mut self) {
        if cache::remove_active_jvm() <= 0 {
            if self.detach_thread_on_drop {
                self.detach_current_thread();
            }
            cache::set_thread_local_env(None);
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
    maven_settings: MavenSettings,
    javafx: bool,
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
            maven_settings: MavenSettings::default(),
            javafx: false,
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

    /// Defines the maven settings to use for provisioning maven artifacts.
    pub fn with_maven_settings(&'a mut self, maven_settings: MavenSettings) -> &'a mut JvmBuilder {
        self.maven_settings = maven_settings;
        self
    }

    /// Adds JavaFX support to the created JVM
    pub fn with_javafx_support(&'a mut self) -> &'a mut JvmBuilder {
        self.javafx = true;
        self
    }

    /// Creates a Jvm
    pub fn build(&mut self) -> errors::Result<Jvm> {
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
            let jassets_path = self.get_jassets_path()?;
            let all_jars = get_dir_content(&jassets_path)?.files;
            // This is the j4rs jar that should be included in the classpath
            let j4rs_jar_to_use = format!("j4rs-{}-jar-with-dependencies.jar", j4rs_version());
            // Filter out possible incorrect jars of j4rs
            let filtered_jars: Vec<String> = all_jars.into_iter()
                .filter(|jar_full_path| {
                    let jarname = jar_full_path.split(MAIN_SEPARATOR).last().unwrap_or(jar_full_path);
                    !jarname.contains("j4rs-") || jarname.ends_with(&j4rs_jar_to_use)
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

        if self.javafx {
            let jassets_path = self.get_jassets_path()?;
            let jassets_path_string = jassets_path.to_str().unwrap_or(".");
            let modules_path = format!("--module-path {}", jassets_path_string);
            jvm_options.push(modules_path);
            jvm_options.push("--add-modules javafx.base,javafx.controls,javafx.graphics,javafx.fxml".to_string());
        }
        self.java_opts.clone().into_iter().for_each(|opt| jvm_options.push(opt.to_string()));

        // Pass to the Java world the name of the j4rs library.
        let lib_name_opt = if self.lib_name_opt.is_none() && !self.skip_setting_native_lib {
            let deps_dir = utils::deps_dir()?;
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

        provisioning::set_maven_settings(&self.maven_settings);

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
        Jvm::new(&[], None)
    }

    fn get_jassets_path(&self) -> errors::Result<PathBuf> {
        match &self.base_path {
            Some(base_path_string) => {
                let mut pb = PathBuf::from(base_path_string);
                pb.push("jassets");
                let mut global_jassets_path_opt = cache::JASSETS_PATH.lock()?;
                *global_jassets_path_opt = Some(pb.clone());
                Ok(pb)
            }
            None => utils::default_jassets_path(),
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
        serialized: bool,
    },
    /// A serialized arg that is created in the Rust world.
    Rust {
        json: String,
        class_name: String,
        serialized: bool,
    },
    /// An non-serialized arg created in the Rust world, that contains a Java instance.
    ///
    /// The instance is a Basic Java type, like Integer, Float, String etc.
    RustBasic {
        instance: Instance,
        class_name: String,
        serialized: bool,
    },
}

impl InvocationArg {
    /// Creates a InvocationArg::Rust.
    /// This is default for the Args that are created from the Rust code.
    pub fn new<T>(arg: &T, class_name: &str) -> InvocationArg
        where T: Serialize + Any
    {
        Self::new_2(
            arg,
            class_name,
            cache::get_thread_local_env().expect("Could not find the jni_env in the local cache. Please make sure that you created a Jvm before using Jvm::new"))
            .expect("Could not create the InvocationArg. Please see the logs/console for more details.")
    }

    pub fn new_2<T>(arg: &T, class_name: &str, jni_env: *mut JNIEnv) -> errors::Result<InvocationArg>
        where T: Serialize + Any
    {
        let arg_any = arg as &dyn Any;
        if let Some(a) = arg_any.downcast_ref::<String>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(jni_utils::global_jobject_from_str(a, jni_env)?, class_name)?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i8>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(jni_utils::global_jobject_from_i8(a, jni_env)?, class_name)?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i16>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(jni_utils::global_jobject_from_i16(a, jni_env)?, class_name)?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i32>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(jni_utils::global_jobject_from_i32(a, jni_env)?, class_name)?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i64>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(jni_utils::global_jobject_from_i64(a, jni_env)?, class_name)?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<f32>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(jni_utils::global_jobject_from_f32(a, jni_env)?, class_name)?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<f64>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(jni_utils::global_jobject_from_f64(a, jni_env)?, class_name)?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else {
            let json = serde_json::to_string(arg)?;
            Ok(InvocationArg::Rust {
                json: json,
                class_name: class_name.to_string(),
                serialized: true,
            })
        }
    }

    fn make_primitive(&mut self) -> errors::Result<()> {
        match utils::primitive_of(self) {
            Some(primitive_repr) => {
                match self {
                    &mut InvocationArg::Java { instance: _, ref mut class_name, serialized: _ } => *class_name = primitive_repr,
                    &mut InvocationArg::Rust { json: _, ref mut class_name, serialized: _ } => *class_name = primitive_repr,
                    &mut InvocationArg::RustBasic { instance: _, ref mut class_name, serialized: _ } => *class_name = primitive_repr,
                };
                Ok(())
            }
            None => Err(errors::J4RsError::JavaError(format!("Cannot transform to primitive: {}", utils::get_class_name(&self))))
        }
    }

    /// Consumes this InvocationArg and transforms it to an InvocationArg that contains a Java primitive, leveraging Java's autoboxing.
    ///
    /// This action can be done by calling `Jvm::cast` of Instances as well (e.g.: jvm.cast(&instance, "int"))
    /// but calling `into_primitive` is faster, as it does not involve JNI calls.
    pub fn into_primitive(self) -> errors::Result<InvocationArg> {
        let mut ia = self;
        ia.make_primitive()?;
        Ok(ia)
    }

    /// Creates a `jobject` from this InvocationArg.
    pub fn as_java_ptr_with_global_ref(&self, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
        match self {
            _s @ &InvocationArg::Java { .. } => jni_utils::invocation_arg_jobject_from_java(&self, jni_env, true),
            _s @ &InvocationArg::Rust { .. } => jni_utils::invocation_arg_jobject_from_rust_serialized(&self, jni_env, true),
            _s @ &InvocationArg::RustBasic { .. } => jni_utils::invocation_arg_jobject_from_rust_basic(&self, jni_env, true),
        }
    }

    /// Creates a `jobject` from this InvocationArg. The jobject contains a local reference.
    pub fn as_java_ptr_with_local_ref(&self, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
        match self {
            _s @ &InvocationArg::Java { .. } => jni_utils::invocation_arg_jobject_from_java(&self, jni_env, false),
            _s @ &InvocationArg::Rust { .. } => jni_utils::invocation_arg_jobject_from_rust_serialized(&self, jni_env, false),
            _s @ &InvocationArg::RustBasic { .. } => jni_utils::invocation_arg_jobject_from_rust_basic(&self, jni_env, false),
        }
    }

    /// Consumes this invocation arg and returns its Instance
    pub fn instance(self) -> errors::Result<Instance> {
        match self {
            InvocationArg::Java { instance: i, .. } => Ok(i),
            InvocationArg::RustBasic { .. } => Err(errors::J4RsError::RustError(format!("Invalid operation: Cannot get the instance of an InvocationArg::RustBasic"))),
            InvocationArg::Rust { .. } => Err(errors::J4RsError::RustError(format!("Cannot get the instance from an InvocationArg::Rust"))),
        }
    }

    pub fn class_name(&self) -> &str {
        match self {
            &InvocationArg::Java { instance: _, ref class_name, serialized: _ } => class_name,
            &InvocationArg::Rust { json: _, ref class_name, serialized: _ } => class_name,
            &InvocationArg::RustBasic { instance: _, ref class_name, serialized: _ } => class_name,
        }
    }

    /// Creates an InvocationArg that contains null
    pub fn create_null(null: Null) -> errors::Result<InvocationArg> {
        let class_name = match null {
            Null::String => CLASS_STRING,
            Null::Boolean => CLASS_BOOLEAN,
            Null::Byte => CLASS_BYTE,
            Null::Character => CLASS_CHARACTER,
            Null::Short => CLASS_SHORT,
            Null::Integer => CLASS_INTEGER,
            Null::Long => CLASS_LONG,
            Null::Float => CLASS_FLOAT,
            Null::Double => CLASS_DOUBLE,
            Null::List => CLASS_LIST,
            Null::Of(class_name) => class_name,
        };
        Ok(InvocationArg::RustBasic {
            instance: Instance::new(ptr::null_mut(), class_name)?,
            class_name: class_name.to_string(),
            serialized: false,
        })
    }
}

/// Represents Java's null. Use this to create null Objects. E.g.:
///
/// let null_integer = InvocationArg::from(Null::Integer);
/// let null_object = InvocationArg::from(Null::Of("some.class.Name"));
pub enum Null<'a> {
    String,
    Boolean,
    Byte,
    Character,
    Short,
    Integer,
    Long,
    Float,
    Double,
    List,
    Of(&'a str),
}

impl From<Instance> for InvocationArg {
    fn from(instance: Instance) -> InvocationArg {
        let class_name = instance.class_name.to_owned();

        InvocationArg::Java {
            instance: instance,
            class_name: class_name,
            serialized: false,
        }
    }
}

impl<'a> TryFrom<Null<'a>> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(null: Null) -> errors::Result<InvocationArg> {
        InvocationArg::create_null(null)
    }
}

impl TryFrom<String> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: String) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_STRING, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [String]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [String]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl<'a> TryFrom<&'a str> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a str) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg.to_string(), CLASS_STRING, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [&'a str]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [&'a str]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.iter().map(|&elem| InvocationArg::try_from(elem)).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<bool> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: bool) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_BOOLEAN, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [bool]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [bool]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i8> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i8) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_BYTE, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [i8]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i8]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<char> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: char) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_CHARACTER, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [char]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [char]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i16> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i16) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_SHORT, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [i16]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i16]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_INTEGER, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [i32]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i32]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_LONG, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [i64]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i64]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<f32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: f32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_FLOAT, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [f32]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [f32]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<f64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: f64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, CLASS_DOUBLE, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [f64]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [f64]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec.into_iter().map(|elem| InvocationArg::try_from(elem.clone())).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl<'a, T: 'static> TryFrom<(&'a [T], &'a str)> for InvocationArg where T: Serialize {
    type Error = errors::J4RsError;
    fn try_from(vec: (&'a [T], &'a str)) -> errors::Result<InvocationArg> {
        let (vec, elements_class_name) = vec;
        let jni_env = cache::get_thread_local_env()?;
        let args: errors::Result<Vec<InvocationArg>> = vec.iter().map(|elem| InvocationArg::new_2(elem, elements_class_name, jni_env)).collect();
        let res = Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<()> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: ()) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, "void", cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a String> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a String) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_STRING, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a bool, > for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a bool) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_BOOLEAN, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a i8> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i8) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_BYTE, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a char> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a char) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_CHARACTER, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a i16> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i16) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_SHORT, cache::get_thread_local_env()?)
    }
}

impl<'a, 'b> TryFrom<&'a i32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_INTEGER, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a i64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_LONG, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a f32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a f32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_FLOAT, cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a f64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a f64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, CLASS_DOUBLE, cache::get_thread_local_env()?)
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
        if self.tx_address > 0 {
            debug("Dropping an InstanceReceiver");
            let p = self.tx_address as *mut Sender<Instance>;
            unsafe {
                let tx = Box::from_raw(p);
                mem::drop(tx);
            }
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
    /// This object is an instance of `org/astonbitecode/j4rs/api/Instance`
    #[serde(skip)]
    pub(crate) jinstance: jobject,
    #[serde(skip)]
    skip_deleting_jobject: bool,
}

impl Instance {
    /// Creates a new Instance, leaving the passed jobject as is.
    /// In most cases, the jobject is already transformed to a global reference.
    pub(crate) fn new(obj: jobject, classname: &str) -> errors::Result<Instance> {
        Ok(Instance {
            jinstance: obj,
            class_name: classname.to_string(),
            skip_deleting_jobject: false,
        })
    }

    /// Returns the class name of this instance
    pub fn class_name(&self) -> &str {
        self.class_name.as_ref()
    }

    /// Consumes the Instance and returns its jobject
    pub fn java_object(mut self) -> jobject {
        self.skip_deleting_jobject = true;
        self.jinstance
    }

    #[deprecated(since = "0.12.0", note = "Please use Instance::from_jobject or Instance::from_jobject_with_global_ref instead")]
    pub fn from(obj: jobject) -> errors::Result<Instance> {
        let _jvm = cache::get_thread_local_env().map_err(|_| {
            Jvm::attach_thread()
        });

        let global = jni_utils::create_global_ref_from_local_ref(obj, cache::get_thread_local_env()?)?;
        Ok(Instance {
            jinstance: global,
            class_name: cache::UNKNOWN_FOR_RUST.to_string(),
            skip_deleting_jobject: false,
        })
    }

    pub fn from_jobject(obj: jobject) -> errors::Result<Instance> {
        let _jvm = cache::get_thread_local_env().map_err(|_| {
            Jvm::attach_thread()
        });

        Ok(Instance {
            jinstance: obj,
            class_name: cache::UNKNOWN_FOR_RUST.to_string(),
            skip_deleting_jobject: false,
        })
    }

    pub fn from_jobject_with_global_ref(obj: jobject) -> errors::Result<Instance> {
        let _jvm = cache::get_thread_local_env().map_err(|_| {
            Jvm::attach_thread()
        });

        let global = jni_utils::create_global_ref_from_local_ref(obj, cache::get_thread_local_env()?)?;
        Ok(Instance {
            jinstance: global,
            class_name: cache::UNKNOWN_FOR_RUST.to_string(),
            skip_deleting_jobject: false,
        })
    }

    /// Creates a weak reference of this Instance.
    fn _weak_ref(&self) -> errors::Result<Instance> {
        Ok(Instance {
            class_name: self.class_name.clone(),
            jinstance: jni_utils::_create_weak_global_ref_from_global_ref(self.jinstance.clone(), cache::get_thread_local_env()?)?,
            skip_deleting_jobject: false,
        })
    }
}

impl TryFrom<InvocationArg> for Instance {
    type Error = errors::J4RsError;
    fn try_from(invocation_arg: InvocationArg) -> errors::Result<Instance> {
        let obj = invocation_arg.as_java_ptr_with_local_ref(cache::get_thread_local_env()?)?;
        Instance::new(obj, invocation_arg.class_name())
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        debug(&format!("Dropping an instance of {}", self.class_name));
        if !self.skip_deleting_jobject {
            if let Some(j_env) = cache::get_thread_local_env_opt() {
                jni_utils::delete_java_ref(j_env, self.jinstance);
            }
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

    fn new_with_instance_ref(instance: &Instance, jvm: &'a Jvm) -> errors::Result<ChainableInstance<'a>> {
        let cloned = jvm.clone_instance(&instance)?;
        Ok(ChainableInstance { instance: cloned, jvm })
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
    pub fn to_rust<T: Any>(self) -> errors::Result<T> where T: DeserializeOwned {
        self.jvm.to_rust(self.instance)
    }

    /// Returns the Rust representation of the provided instance, boxed
    pub fn to_rust_boxed<T: Any>(self) -> errors::Result<Box<T>> where T: DeserializeOwned {
        self.jvm.to_rust_boxed(self.instance)
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
    use serde::Deserialize;
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
        let _jvm = JvmBuilder::new().build().unwrap();
        let _ = InvocationArg::new(&"something".to_string(), "somethingelse");

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
    fn invocation_arg_try_from_basic_types() {
        let _jvm = JvmBuilder::new().build().unwrap();
        validate_type(InvocationArg::try_from("str").unwrap(), "java.lang.String");
        validate_type(InvocationArg::try_from("str".to_string()).unwrap(), "java.lang.String");
        validate_type(InvocationArg::try_from(true).unwrap(), "java.lang.Boolean");
        validate_type(InvocationArg::try_from(1_i8).unwrap(), "java.lang.Byte");
        validate_type(InvocationArg::try_from('c').unwrap(), "java.lang.Character");
        validate_type(InvocationArg::try_from(1_i16).unwrap(), "java.lang.Short");
        validate_type(InvocationArg::try_from(1_i64).unwrap(), "java.lang.Long");
        validate_type(InvocationArg::try_from(0.1_f32).unwrap(), "java.lang.Float");
        validate_type(InvocationArg::try_from(0.1_f64).unwrap(), "java.lang.Double");
        validate_type(InvocationArg::try_from(()).unwrap(), "void");

        validate_type(InvocationArg::try_from(&"str".to_string()).unwrap(), "java.lang.String");
        validate_type(InvocationArg::try_from(&true).unwrap(), "java.lang.Boolean");
        validate_type(InvocationArg::try_from(&1_i8).unwrap(), "java.lang.Byte");
        validate_type(InvocationArg::try_from(&'c').unwrap(), "java.lang.Character");
        validate_type(InvocationArg::try_from(&1_i16).unwrap(), "java.lang.Short");
        validate_type(InvocationArg::try_from(&1_i64).unwrap(), "java.lang.Long");
        validate_type(InvocationArg::try_from(&0.1_f32).unwrap(), "java.lang.Float");
        validate_type(InvocationArg::try_from(&0.1_f64).unwrap(), "java.lang.Double");
    }

    #[test]
    fn invocation_into_primitive() {
        let _jvm: Jvm = JvmBuilder::new().build().unwrap();
        assert!(InvocationArg::try_from(false).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from(1_i8).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from(1_i16).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from(1_32).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from(1_i64).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from(0.1_f32).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from(0.1_f64).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from('c').unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from(()).unwrap().into_primitive().is_ok());
        assert!(InvocationArg::try_from("string").unwrap().into_primitive().is_err());
    }

    #[test]
    fn test_copy_j4rs_libs_under() {
        let newdir = "./newdir";
        Jvm::copy_j4rs_libs_under(newdir).unwrap();

        let _ = fs_extra::remove_items(&vec![newdir]);
    }

    #[test]
    fn test_select() {
        let (tx1, rx1) = channel();
        let ir1 = InstanceReceiver::new(rx1, 0);
        let (_tx2, rx2) = channel();
        let ir2 = InstanceReceiver::new(rx2, 0);
        let (tx3, rx3) = channel();
        let ir3 = InstanceReceiver::new(rx3, 0);

        thread::spawn(move || {
            let _ = tx3.send(Instance::new(ptr::null_mut(), CLASS_STRING).unwrap());
            // Block the thread as sending does not block the current thread
            thread::sleep(time::Duration::from_millis(10));
            let _ = tx1.send(Instance::new(ptr::null_mut(), CLASS_STRING).unwrap());
            thread::sleep(time::Duration::from_millis(10));
            let _ = tx3.send(Instance::new(ptr::null_mut(), CLASS_STRING).unwrap());
        });

        let (index1, _) = Jvm::select(&[&ir1, &ir2, &ir3]).unwrap();
        let (index2, _) = Jvm::select(&[&ir1, &ir2, &ir3]).unwrap();
        let (index3, _) = Jvm::select(&[&ir1, &ir2, &ir3]).unwrap();
        assert!(index1 == 2);
        assert!(index2 == 0);
        assert!(index3 == 2);
    }

    #[test]
    fn test_select_timeout() {
        let (tx1, rx1) = channel();
        let ir1 = InstanceReceiver::new(rx1, 0);
        let (tx2, rx2) = channel();
        let ir2 = InstanceReceiver::new(rx2, 0);

        thread::spawn(move || {
            let _ = tx1.send(Instance::new(ptr::null_mut(), CLASS_STRING).unwrap());
            // Block the thread as sending does not block the current thread
            thread::sleep(time::Duration::from_millis(10));
            let _ = tx2.send(Instance::new(ptr::null_mut(), CLASS_STRING).unwrap());
        });

        let d = time::Duration::from_millis(500);
        let (index1, _) = Jvm::select_timeout(&[&ir1, &ir2], &d).unwrap();
        let (index2, _) = Jvm::select_timeout(&[&ir1, &ir2], &d).unwrap();
        assert!(Jvm::select_timeout(&[&ir1, &ir2], &d).is_err());
        dbg!(index1);
        dbg!(index2);
        assert!(index1 == 0);
        assert!(index2 == 1);
    }

    fn validate_type(ia: InvocationArg, class: &str) {
        let b = match ia {
            _s @ InvocationArg::Java { .. } => false,
            InvocationArg::Rust { class_name, json: _, .. } => {
                class == class_name
            }
            InvocationArg::RustBasic { instance: _, class_name, serialized: _ } => {
                class == class_name
            }
        };
        assert!(b);
    }
}