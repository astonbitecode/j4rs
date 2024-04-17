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

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::ops::Drop;
use std::os::raw::c_void;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::ptr;
use std::sync::mpsc::channel;
use std::{fs, thread, time};
use std::borrow::Borrow;

use fs_extra::dir::get_dir_content;
use jni_sys::{
    self, jint, jobject, jsize, jstring, JNIEnv, JavaVM, JavaVMInitArgs, JavaVMOption,
    JNI_EDETACHED, JNI_EEXIST, JNI_EINVAL, JNI_ENOMEM, JNI_ERR, JNI_EVERSION, JNI_OK, JNI_TRUE,
    JNI_VERSION_1_6,
};
use libc::c_char;
use serde::de::DeserializeOwned;
use serde_json;

use instance::{ChainableInstance, Instance, InstanceReceiver};

use crate::errors;
use crate::errors::{opt_to_res, J4RsError};
use crate::jni_utils;
use crate::provisioning;
use crate::provisioning::{get_maven_settings, JavaArtifact, LocalJarArtifact, MavenArtifact};
use crate::utils;
use crate::{api_tweaks as tweaks, cache, InvocationArg, MavenSettings};

use super::logger::{debug, error, info, warn};

pub(crate) mod instance;
pub(crate) mod invocation_arg;

// Initialize the environment
include!(concat!(env!("OUT_DIR"), "/j4rs_init.rs"));

const CLASS_STRING: &'static str = "java.lang.String";
const CLASS_BOOLEAN: &'static str = "java.lang.Boolean";
const CLASS_BYTE: &'static str = "java.lang.Byte";
const CLASS_CHARACTER: &'static str = "java.lang.Character";
const CLASS_SHORT: &'static str = "java.lang.Short";
const CLASS_INTEGER: &'static str = "java.lang.Integer";
const CLASS_LONG: &'static str = "java.lang.Long";
const CLASS_FLOAT: &'static str = "java.lang.Float";
const CLASS_DOUBLE: &'static str = "java.lang.Double";
const CLASS_LIST: &'static str = "java.util.List";
pub(crate) const PRIMITIVE_BOOLEAN: &'static str = "boolean";
pub(crate) const PRIMITIVE_BYTE: &'static str = "byte";
pub(crate) const PRIMITIVE_SHORT: &'static str = "short";
pub(crate) const PRIMITIVE_INT: &'static str = "int";
pub(crate) const PRIMITIVE_LONG: &'static str = "long";
pub(crate) const PRIMITIVE_FLOAT: &'static str = "float";
pub(crate) const PRIMITIVE_DOUBLE: &'static str = "double";
pub(crate) const PRIMITIVE_CHAR: &'static str = "char";

pub(crate) const PRIMITIVE_BOOLEAN_ARRAY: &'static str = "[Z";
pub(crate) const PRIMITIVE_BYTE_ARRAY: &'static str = "[B";
pub(crate) const PRIMITIVE_SHORT_ARRAY: &'static str = "[S";
pub(crate) const PRIMITIVE_INT_ARRAY: &'static str = "[I";
pub(crate) const PRIMITIVE_LONG_ARRAY: &'static str = "[J";
pub(crate) const PRIMITIVE_FLOAT_ARRAY: &'static str = "[F";
pub(crate) const PRIMITIVE_DOUBLE_ARRAY: &'static str = "[D";
pub(crate) const PRIMITIVE_CHAR_ARRAY: &'static str = "[C";

pub(crate) const CLASS_NATIVE_CALLBACK_TO_RUST_CHANNEL_SUPPORT: &'static str =
    "org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport";
pub(crate) const CLASS_J4RS_EVENT_HANDLER: &'static str =
    "org.astonbitecode.j4rs.api.jfx.handlers.J4rsEventHandler";
pub(crate) const CLASS_J4RS_FXML_LOADER: &'static str =
    "org.astonbitecode.j4rs.api.jfx.J4rsFxmlLoader";
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
            debug("A JVM is already created for this thread. Retrieving it...");
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
                let mut cstrings_to_drop: Vec<*mut c_char> = Vec::with_capacity(jvm_options.len());
                let mut jvm_options_vec: Vec<JavaVMOption> = jvm_options
                    .iter()
                    .map(|opt| {
                        let cstr = utils::to_c_string(opt);
                        let jo = JavaVMOption {
                            optionString: cstr,
                            extraInfo: ptr::null_mut() as *mut c_void,
                        };
                        cstrings_to_drop.push(cstr);
                        jo
                    })
                    .collect();

                let mut jvm_arguments = JavaVMInitArgs {
                    version: JNI_VERSION_1_6,
                    nOptions: jvm_options.len() as i32,
                    options: jvm_options_vec.as_mut_ptr(),
                    ignoreUnrecognized: JNI_TRUE,
                };

                let int_result = tweaks::create_java_vm(
                    &mut jvm,
                    (&mut jni_environment as *mut *mut JNIEnv) as *mut *mut c_void,
                    (&mut jvm_arguments as *mut JavaVMInitArgs) as *mut c_void,
                );

                cstrings_to_drop
                    .into_iter()
                    .for_each(|cstr| utils::drop_c_string(cstr));

                int_result
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

            Err(J4RsError::JavaError(
                format!("Could not create the JVM: {}", error_message).to_string(),
            ))
        } else {
            let jvm = unsafe { Self::try_from(jni_environment)? };
            if let Some(libname) = lib_name_to_load {
                // Pass to the Java world the name of the j4rs library.
                debug(&format!(
                    "Initializing NativeCallbackSupport with libname {}",
                    libname
                ));
                jvm.invoke_static(
                    CLASS_NATIVE_CALLBACK_TO_RUST_CHANNEL_SUPPORT,
                    "initialize",
                    &vec![InvocationArg::try_from(libname)?],
                )?;
                debug("NativeCallbackSupport initialized");
            }

            Ok(jvm)
        }
    }

    pub unsafe fn try_from(jni_environment: *mut JNIEnv) -> errors::Result<Jvm> {
        if cache::get_thread_local_env_opt().is_none() {
            // Create and set the environment in Thread Local
            let _ = cache::get_jni_get_method_id().or_else(|| {
                cache::set_jni_get_method_id(Some((**jni_environment).v1_6.GetMethodID))
            });
            let _ = cache::get_jni_get_static_method_id().or_else(|| {
                cache::set_jni_get_static_method_id(Some(
                    (**jni_environment).v1_6.GetStaticMethodID,
                ))
            });
            let _ = cache::get_jni_new_object()
                .or_else(|| cache::set_jni_new_object(Some((**jni_environment).v1_6.NewObject)));
            let _ = cache::get_jni_new_string_utf().or_else(|| {
                cache::set_jni_new_string_utf(Some((**jni_environment).v1_6.NewStringUTF))
            });
            let _ = cache::get_jni_get_string_utf_chars().or_else(|| {
                cache::set_jni_get_string_utf_chars(Some(
                    (**jni_environment).v1_6.GetStringUTFChars,
                ))
            });
            let _ = cache::get_jni_release_string_utf_chars().or_else(|| {
                cache::set_jni_release_string_utf_chars(Some(
                    (**jni_environment).v1_6.ReleaseStringUTFChars,
                ))
            });
            let _ = cache::get_jni_call_object_method().or_else(|| {
                cache::set_jni_call_object_method(Some((**jni_environment).v1_6.CallObjectMethod))
            });
            let _ = cache::get_jni_call_byte_method().or_else(|| {
                cache::set_jni_call_byte_method(Some((**jni_environment).v1_6.CallByteMethod))
            });
            let _ = cache::get_jni_call_short_method().or_else(|| {
                cache::set_jni_call_short_method(Some((**jni_environment).v1_6.CallShortMethod))
            });
            let _ = cache::get_jni_call_int_method().or_else(|| {
                cache::set_jni_call_int_method(Some((**jni_environment).v1_6.CallIntMethod))
            });
            let _ = cache::get_jni_call_long_method().or_else(|| {
                cache::set_jni_call_long_method(Some((**jni_environment).v1_6.CallLongMethod))
            });
            let _ = cache::get_jni_call_float_method().or_else(|| {
                cache::set_jni_call_float_method(Some((**jni_environment).v1_6.CallFloatMethod))
            });
            let _ = cache::get_jni_call_double_method().or_else(|| {
                cache::set_jni_call_double_method(Some((**jni_environment).v1_6.CallDoubleMethod))
            });
            let _ = cache::get_jni_call_void_method().or_else(|| {
                cache::set_jni_call_void_method(Some((**jni_environment).v1_6.CallVoidMethod))
            });
            let _ = cache::get_jni_call_static_object_method().or_else(|| {
                cache::set_jni_call_static_object_method(Some(
                    (**jni_environment).v1_6.CallStaticObjectMethod,
                ))
            });
            let _ = cache::get_jni_get_array_length().or_else(|| {
                cache::set_jni_get_array_length(Some(
                    (**jni_environment).v1_6.GetArrayLength,
                ))
            });
            let _ = cache::get_jni_get_byte_array_elements().or_else(|| {
                cache::set_jni_get_byte_array_elements(Some(
                    (**jni_environment).v1_6.GetByteArrayElements,
                ))
            });
            let _ = cache::get_jni_release_byte_array_elements().or_else(|| {
                cache::set_jni_release_byte_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseByteArrayElements,
                ))
            });
            let _ = cache::get_jni_get_short_array_elements().or_else(|| {
                cache::set_jni_get_short_array_elements(Some(
                    (**jni_environment).v1_6.GetShortArrayElements,
                ))
            });
            let _ = cache::get_jni_release_short_array_elements().or_else(|| {
                cache::set_jni_release_short_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseShortArrayElements,
                ))
            });
            let _ = cache::get_jni_get_char_array_elements().or_else(|| {
                cache::set_jni_get_char_array_elements(Some(
                    (**jni_environment).v1_6.GetCharArrayElements,
                ))
            });
            let _ = cache::get_jni_release_char_array_elements().or_else(|| {
                cache::set_jni_release_char_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseCharArrayElements,
                ))
            });
            let _ = cache::get_jni_get_int_array_elements().or_else(|| {
                cache::set_jni_get_int_array_elements(Some(
                    (**jni_environment).v1_6.GetIntArrayElements,
                ))
            });
            let _ = cache::get_jni_release_int_array_elements().or_else(|| {
                cache::set_jni_release_int_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseIntArrayElements,
                ))
            });
            let _ = cache::get_jni_get_long_array_elements().or_else(|| {
                cache::set_jni_get_long_array_elements(Some(
                    (**jni_environment).v1_6.GetLongArrayElements,
                ))
            });
            let _ = cache::get_jni_release_long_array_elements().or_else(|| {
                cache::set_jni_release_long_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseLongArrayElements,
                ))
            });
            let _ = cache::get_jni_get_float_array_elements().or_else(|| {
                cache::set_jni_get_float_array_elements(Some(
                    (**jni_environment).v1_6.GetFloatArrayElements,
                ))
            });
            let _ = cache::get_jni_release_float_array_elements().or_else(|| {
                cache::set_jni_release_float_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseFloatArrayElements,
                ))
            });
            let _ = cache::get_jni_get_double_array_elements().or_else(|| {
                cache::set_jni_get_double_array_elements(Some(
                    (**jni_environment).v1_6.GetDoubleArrayElements,
                ))
            });
            let _ = cache::get_jni_release_double_array_elements().or_else(|| {
                cache::set_jni_release_double_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseDoubleArrayElements,
                ))
            });
            let _ = cache::get_jni_get_boolean_array_elements().or_else(|| {
                cache::set_jni_get_boolean_array_elements(Some(
                    (**jni_environment).v1_6.GetBooleanArrayElements,
                ))
            });
            let _ = cache::get_jni_release_boolean_array_elements().or_else(|| {
                cache::set_jni_release_boolean_array_elements(Some(
                    (**jni_environment).v1_6.ReleaseBooleanArrayElements,
                ))
            });
            let _ = cache::get_jni_new_object_array().or_else(|| {
                cache::set_jni_new_object_array(Some((**jni_environment).v1_6.NewObjectArray))
            });
            let _ = cache::get_jni_set_object_array_element().or_else(|| {
                cache::set_jni_set_object_array_element(Some(
                    (**jni_environment).v1_6.SetObjectArrayElement,
                ))
            });
            let ec = cache::get_jni_exception_check().or_else(|| {
                cache::set_jni_exception_check(Some((**jni_environment).v1_6.ExceptionCheck))
            });
            let ed = cache::get_jni_exception_describe().or_else(|| {
                cache::set_jni_exception_describe(Some((**jni_environment).v1_6.ExceptionDescribe))
            });
            let exclear = cache::get_jni_exception_clear().or_else(|| {
                cache::set_jni_exception_clear(Some((**jni_environment).v1_6.ExceptionClear))
            });
            let _ = cache::get_jni_delete_local_ref().or_else(|| {
                cache::set_jni_delete_local_ref(Some((**jni_environment).v1_6.DeleteLocalRef))
            });
            let _ = cache::get_jni_delete_global_ref().or_else(|| {
                cache::set_jni_delete_global_ref(Some((**jni_environment).v1_6.DeleteGlobalRef))
            });
            let _ = cache::get_jni_new_global_ref().or_else(|| {
                cache::set_jni_new_global_ref(Some((**jni_environment).v1_6.NewGlobalRef))
            });
            let _ = cache::get_jni_throw_new()
                .or_else(|| cache::set_jni_throw_new(Some((**jni_environment).v1_6.ThrowNew)));
            let _ = cache::get_is_same_object()
                .or_else(|| cache::set_is_same_object(Some((**jni_environment).v1_6.IsSameObject)));

            match (ec, ed, exclear) {
                (Some(ec), Some(ed), Some(exclear)) => {
                    if (ec)(jni_environment) == JNI_TRUE {
                        (ed)(jni_environment);
                        (exclear)(jni_environment);
                        Err(J4RsError::JavaError(
                            "The VM cannot be started... Please check the logs.".to_string(),
                        ))
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
                (_, _, _) => Err(J4RsError::JniError(format!(
                    "Could not initialize the JVM: Error while trying to retrieve JNI functions."
                ))),
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
    pub fn create_instance(
        &self,
        class_name: &str,
        inv_args: &[impl Borrow<InvocationArg>],
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Instantiating class {} using {} arguments",
            class_name,
            inv_args.len()
        ));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;

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
                let inv_arg_java =
                    (&inv_args[i as usize]).borrow().as_java_ptr_with_global_ref(self.jni_env)?;
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

            let java_instance_global_instance =
                jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            jni_utils::delete_java_ref(self.jni_env, array_ptr);
            jni_utils::delete_java_ref(self.jni_env, class_name_jstring);
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(self.jni_env, inv_arg_jobject);
            }

            // Create and return the Instance
            Self::do_return(
                self.jni_env,
                Instance {
                    jinstance: java_instance_global_instance,
                    class_name: class_name.to_string(),
                    skip_deleting_jobject: false,
                },
            )
        }
    }

    /// Retrieves the static class `class_name`.
    pub fn static_class(&self, class_name: &str) -> errors::Result<Instance> {
        debug(&format!("Retrieving static class {}", class_name));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;

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
            Self::do_return(
                self.jni_env,
                Instance::from_jobject_with_global_ref(java_instance)?,
            )
        }
    }

    /// Creates a new Java Array with elements of the class `class_name`.
    /// The array will have the `InvocationArg`s populated.
    /// The `InvocationArg`s __must__ be of type _class_name_.
    pub fn create_java_array(
        &self,
        class_name: &str,
        inv_args: &[impl Borrow<InvocationArg>],
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Creating a java array of class {} with {} elements",
            class_name,
            inv_args.len()
        ));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;

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
                let inv_arg_java =
                    inv_args[i as usize].borrow().as_java_ptr_with_global_ref(self.jni_env)?;
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

            let java_instance_global_instance =
                jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(self.jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(self.jni_env, array_ptr);
            jni_utils::delete_java_ref(self.jni_env, class_name_jstring);

            // Create and return the Instance
            Self::do_return(
                self.jni_env,
                Instance {
                    jinstance: java_instance_global_instance,
                    class_name: class_name.to_string(),
                    skip_deleting_jobject: false,
                },
            )
        }
    }

    /// Creates a new Java List with elements of the class `class_name`.
    /// The array will have the `InvocationArg`s populated.
    /// The `InvocationArg`s __must__ be of type _class_name_.
    #[deprecated(since = "0.15.0", note = "Please use `java_list` instead")]
    pub fn create_java_list(
        &self,
        class_name: &str,
        inv_args: &[InvocationArg],
    ) -> errors::Result<Instance> {
        Jvm::do_create_java_list(self.jni_env, class_name, inv_args)
    }

    /// Creates a new Java List with elements of the class `inner_class_name`.
    pub fn java_list<'a>(
        &self,
        inner_class_name: impl Into<&'a str>,
        inv_args: Vec<impl TryInto<InvocationArg, Error=J4RsError>>,
    ) -> errors::Result<Instance> {
        let v: Result<Vec<InvocationArg>, J4RsError> =
            inv_args.into_iter().map(|arg| arg.try_into()).collect();
        Self::do_create_java_list(self.jni_env, inner_class_name.into(), v?.as_ref())
    }

    fn do_create_java_list(
        jni_env: *mut JNIEnv,
        class_name: &str,
        inv_args: &[InvocationArg],
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Creating a java list of class {} with {} elements",
            class_name,
            inv_args.len()
        ));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&class_name, jni_env)?;

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

            let java_instance_global_instance =
                jni_utils::create_global_ref_from_local_ref(java_instance, jni_env)?;
            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(jni_env, array_ptr);
            jni_utils::delete_java_ref(jni_env, class_name_jstring);

            // Create and return the Instance
            Self::do_return(
                jni_env,
                Instance {
                    jinstance: java_instance_global_instance,
                    class_name: class_name.to_string(),
                    skip_deleting_jobject: false,
                },
            )
        }
    }

    /// Creates a new Java Map with keys of class `key_class_name` and values of class `value_class_name`.
    pub fn java_map<'a>(
        &self,
        key_class_name: impl Into<&'a str>,
        value_class_name: impl Into<&'a str>,
        inv_args: HashMap<
            impl TryInto<InvocationArg, Error=J4RsError>,
            impl TryInto<InvocationArg, Error=J4RsError>,
        >,
    ) -> errors::Result<Instance> {
        let mut inv_args_results: Vec<Result<InvocationArg, J4RsError>> =
            Vec::with_capacity(inv_args.len() * 2);
        let mut i = 0;
        let mut inv_args = inv_args;

        for (key, val) in inv_args.drain() {
            inv_args_results.insert(i, key.try_into());
            i = i + 1;
            inv_args_results.insert(i, val.try_into());
            i = i + 1;
        }
        let inv_args: Result<Vec<InvocationArg>, J4RsError> = inv_args_results
            .into_iter()
            .map(|arg| arg.try_into())
            .collect();
        Self::do_create_java_map(
            self.jni_env,
            key_class_name.into(),
            value_class_name.into(),
            inv_args?.as_ref(),
        )
    }

    fn do_create_java_map(
        jni_env: *mut JNIEnv,
        key_class_name: &str,
        value_class_name: &str,
        inv_args: &[InvocationArg],
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Creating a java map with keys of class {} and values of class {} with {} elements",
            key_class_name,
            value_class_name,
            inv_args.len() / 2
        ));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the key_class_name
            let key_class_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&key_class_name, jni_env)?;
            // Factory invocation - second argument: create a jstring to pass as argument for the value_class_name
            let value_class_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&value_class_name, jni_env)?;

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
            // Call the method of the factory that instantiates a new Java Map with keys of `key_class_name`
            // and values of `value_class_name`.
            // This returns a Instance that acts like a proxy to the Java world.
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                jni_env,
                cache::get_factory_class()?,
                cache::get_factory_create_java_map_method()?,
                key_class_name_jstring,
                value_class_name_jstring,
                array_ptr,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(jni_env, ())?;

            let java_instance_global_instance =
                jni_utils::create_global_ref_from_local_ref(java_instance, jni_env)?;
            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(jni_env, array_ptr);
            jni_utils::delete_java_ref(jni_env, value_class_name_jstring);
            jni_utils::delete_java_ref(jni_env, key_class_name_jstring);

            // Create and return the Instance
            Self::do_return(
                jni_env,
                Instance {
                    jinstance: java_instance_global_instance,
                    class_name: "".to_string(),
                    skip_deleting_jobject: false,
                },
            )
        }
    }

    /// Invokes the method `method_name` of a created `Instance`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke(
        &self,
        instance: &Instance,
        method_name: &str,
        inv_args: &[impl Borrow<InvocationArg>],
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Invoking method {} of class {} using {} arguments",
            method_name,
            instance.class_name,
            inv_args.len()
        ));
        unsafe {
            // First argument: create a jstring to pass as argument for the method_name
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
                    inv_args[i as usize].borrow().as_java_ptr_with_global_ref(self.jni_env)?;
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

            let java_instance_global_instance =
                jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            for inv_arg_jobject in inv_arg_jobjects {
                jni_utils::delete_java_ref(self.jni_env, inv_arg_jobject);
            }
            jni_utils::delete_java_ref(self.jni_env, array_ptr);
            jni_utils::delete_java_ref(self.jni_env, method_name_jstring);

            // Create and return the Instance
            Self::do_return(
                self.jni_env,
                Instance {
                    jinstance: java_instance_global_instance,
                    class_name: cache::UNKNOWN_FOR_RUST.to_string(),
                    skip_deleting_jobject: false,
                },
            )
        }
    }

    /// Retrieves the field `field_name` of a created `Instance`.
    pub fn field(&self, instance: &Instance, field_name: &str) -> errors::Result<Instance> {
        debug(&format!(
            "Retrieving field {} of class {}",
            field_name, instance.class_name
        ));
        unsafe {
            // First argument: create a jstring to pass as argument for the field_name
            let field_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&field_name, self.jni_env)?;

            // Call the method of the instance
            let java_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_field_method()?,
                field_name_jstring,
            );

            // Check for exceptions before creating the globalref
            Self::do_return(self.jni_env, ())?;

            let java_instance_global_instance =
                jni_utils::create_global_ref_from_local_ref(java_instance, self.jni_env)?;
            // Prevent memory leaks from the created local references
            jni_utils::delete_java_ref(self.jni_env, field_name_jstring);

            // Create and return the Instance
            Self::do_return(
                self.jni_env,
                Instance {
                    jinstance: java_instance_global_instance,
                    class_name: cache::UNKNOWN_FOR_RUST.to_string(),
                    skip_deleting_jobject: false,
                },
            )
        }
    }

    /// Retrieves the field `field_name` of a static class.
    pub fn static_class_field(
        &self,
        class_name: &str,
        field_name: &str,
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Retrieving field {} of static class {}",
            field_name, class_name
        ));
        let i = self.static_class(class_name)?;
        self.field(&i, &field_name)
    }

    /// Invokes the method `method_name` of a created `Instance`, passing an array of `InvocationArg`s.
    /// It returns a Result of `InstanceReceiver` that may be used to get an underlying `Receiver<Instance>`. The result of the invocation will come via this Receiver.
    pub fn invoke_to_channel(
        &self,
        instance: &Instance,
        method_name: &str,
        inv_args: &[impl Borrow<InvocationArg>],
    ) -> errors::Result<InstanceReceiver> {
        debug(&format!("Invoking method {} of class {} using {} arguments. The result of the invocation will come via an InstanceReceiver", method_name, instance.class_name, inv_args.len()));
        unsafe {
            // Create the channel
            let (sender, rx) = channel();
            let tx = Box::new(sender);
            // First argument: the address of the channel Sender
            let raw_ptr = Box::into_raw(tx);
            // Find the address of tx
            let address_string = format!("{:p}", raw_ptr);
            let address = u64::from_str_radix(&address_string[2..], 16).unwrap();

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
                    inv_args[i as usize].borrow().as_java_ptr_with_global_ref(self.jni_env)?;
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
            let address = u64::from_str_radix(&address_string[2..], 16).unwrap();

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
    pub fn invoke_static(
        &self,
        class_name: &str,
        method_name: &str,
        inv_args: &[impl Borrow<InvocationArg>],
    ) -> errors::Result<Instance> {
        debug(&format!(
            "Invoking static method {} of class {} using {} arguments",
            method_name,
            class_name,
            inv_args.len()
        ));
        unsafe {
            // Factory invocation - first argument: create a jstring to pass as argument for the class_name
            let class_name_jstring: jstring =
                jni_utils::global_jobject_from_str(&class_name, self.jni_env)?;
            // Call the method of the factory that creates a Instance for static calls to methods of class `class_name`.
            // This returns a Instance that acts like a proxy to the Java world.
            let java_instance = (opt_to_res(cache::get_jni_call_static_object_method())?)(
                self.jni_env,
                cache::get_factory_class()?,
                cache::get_factory_create_for_static_method()?,
                class_name_jstring,
            );

            // First argument: create a jstring to pass as argument for the method_name
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
                    inv_args[i as usize].borrow().as_java_ptr_with_global_ref(self.jni_env)?;
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
            Self::do_return(
                self.jni_env,
                Instance::from_jobject_with_global_ref(java_instance)?,
            )
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
            Self::do_return(
                self.jni_env,
                Instance::from_jobject_with_global_ref(java_instance)?,
            )
        }
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn cast(&self, from_instance: &Instance, to_class: &str) -> errors::Result<Instance> {
        debug(&format!("Casting to class {}", to_class));
        unsafe {
            // First argument is the jobject that is inside the from_instance
            // Second argument: create a jstring to pass as argument for the to_class
            let to_class_jstring: jstring =
                jni_utils::global_jobject_from_str(&to_class, self.jni_env)?;

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
            Self::do_return(
                self.jni_env,
                Instance::from_jobject_with_global_ref(java_instance)?,
            )
        }
    }

    /// Returns the Rust representation of the provided instance, boxed
    pub fn to_rust_boxed<T>(&self, instance: Instance) -> errors::Result<Box<T>>
        where
            T: DeserializeOwned + Any,
    {
        // Define the macro inside the function in order to have access to &self
        macro_rules! rust_box_from_java_object {
            ($jni_transformation:path) => {{
                // Call the getObjectMethod. This returns a localref
                let object_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                    self.jni_env,
                    instance.jinstance,
                    cache::get_get_object_method()?,
                );
                let object_instance =
                    jni_utils::create_global_ref_from_local_ref(object_instance, self.jni_env)?;
                let v = Box::new($jni_transformation(object_instance, self.jni_env)?);
                let v_any = v as Box<dyn Any>;

                jni_utils::delete_java_ref(self.jni_env, object_instance);

                match v_any.downcast::<T>() {
                    Ok(v) => Ok(v),
                    Err(error) => Err(errors::J4RsError::RustError(format!(
                        "Could not downcast to Rust type: {:?}",
                        error
                    ))),
                }
            }};
        }

        let t_type = TypeId::of::<T>();
        let to_ret = unsafe {
            // Call the getClassName method. This returns a localref
            let object_class_name_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
                self.jni_env,
                instance.jinstance,
                cache::get_get_object_class_name_method()?,
            );
            let object_class_name_instance = jni_utils::create_global_ref_from_local_ref(
                object_class_name_instance,
                self.jni_env,
            )?;
            let ref class_name =
                jni_utils::string_from_jobject(object_class_name_instance, self.jni_env)?;
            jni_utils::delete_java_ref(self.jni_env, object_class_name_instance);
            if t_type == TypeId::of::<String>() && JavaClass::String.get_class_str() == class_name {
                rust_box_from_java_object!(jni_utils::string_from_jobject)
            } else if t_type == TypeId::of::<i32>()
                && (JavaClass::Integer.get_class_str() == class_name || PRIMITIVE_INT == class_name)
            {
                rust_box_from_java_object!(jni_utils::i32_from_jobject)
            } else if t_type == TypeId::of::<i8>()
                && (JavaClass::Byte.get_class_str() == class_name || PRIMITIVE_BYTE == class_name)
            {
                rust_box_from_java_object!(jni_utils::i8_from_jobject)
            } else if t_type == TypeId::of::<i16>()
                && (JavaClass::Short.get_class_str() == class_name || PRIMITIVE_SHORT == class_name)
            {
                rust_box_from_java_object!(jni_utils::i16_from_jobject)
            } else if t_type == TypeId::of::<i64>()
                && (JavaClass::Long.get_class_str() == class_name || PRIMITIVE_LONG == class_name)
            {
                rust_box_from_java_object!(jni_utils::i64_from_jobject)
            } else if t_type == TypeId::of::<f32>()
                && (JavaClass::Float.get_class_str() == class_name || PRIMITIVE_FLOAT == class_name)
            {
                rust_box_from_java_object!(jni_utils::f32_from_jobject)
            } else if t_type == TypeId::of::<f64>()
                && (JavaClass::Double.get_class_str() == class_name
                || PRIMITIVE_DOUBLE == class_name)
            {
                rust_box_from_java_object!(jni_utils::f64_from_jobject)
            } else if t_type == TypeId::of::<Vec<i8>>()
                && PRIMITIVE_BYTE_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::i8_array_from_jobject)
            } else if t_type == TypeId::of::<Vec<i16>>()
                && PRIMITIVE_SHORT_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::i16_array_from_jobject)
            } else if t_type == TypeId::of::<Vec<u16>>()
                && PRIMITIVE_CHAR_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::u16_array_from_jobject)
            } else if t_type == TypeId::of::<Vec<i32>>()
                && PRIMITIVE_INT_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::i32_array_from_jobject)
            } else if t_type == TypeId::of::<Vec<i64>>()
                && PRIMITIVE_LONG_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::i64_array_from_jobject)
            } else if t_type == TypeId::of::<Vec<f32>>()
                && PRIMITIVE_FLOAT_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::f32_array_from_jobject)
            } else if t_type == TypeId::of::<Vec<f64>>()
                && PRIMITIVE_DOUBLE_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::f64_array_from_jobject)
            } else if t_type == TypeId::of::<Vec<bool>>()
                && PRIMITIVE_BOOLEAN_ARRAY == class_name
            {
                rust_box_from_java_object!(jni_utils::boolean_array_from_jobject)
            } else {
                Ok(Box::new(self.to_rust_deserialized(instance)?))
            }
        };

        to_ret
    }

    /// Returns the Rust representation of the provided instance
    pub fn to_rust<T>(&self, instance: Instance) -> errors::Result<T>
        where
            T: DeserializeOwned + Any,
    {
        self.to_rust_boxed(instance).map(|v| *v)
    }

    pub fn to_rust_deserialized<T>(&self, instance: Instance) -> errors::Result<T>
        where
            T: DeserializeOwned + Any,
    {
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
            let global_json_instance =
                jni_utils::create_global_ref_from_local_ref(json_instance, self.jni_env)?;
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
                        InvocationArg::try_from(&maven_artifact.base)?,
                    ],
                )?;

                let res = self.invoke(
                    &instance,
                    "deploy",
                    &vec![
                        InvocationArg::try_from(&maven_artifact.group)?,
                        InvocationArg::try_from(&maven_artifact.id)?,
                        InvocationArg::try_from(&maven_artifact.version)?,
                        InvocationArg::try_from(&maven_artifact.qualifier)?,
                    ],
                );

                if res.is_ok() {
                    break;
                }
            }

            Ok(())
        } else if let Some(local_jar_artifact) = artifact.downcast_ref::<LocalJarArtifact>() {
            let instance = self.create_instance(
                "org.astonbitecode.j4rs.api.deploy.FileSystemDeployer",
                &vec![InvocationArg::try_from(&local_jar_artifact.base)?],
            )?;

            let _ = self.invoke(
                &instance,
                "deploy",
                &vec![InvocationArg::try_from(&local_jar_artifact.path)?],
            )?;
            Ok(())
        } else {
            Err(J4RsError::GeneralError(format!(
                "Don't know how to deploy artifacts of {:?}",
                artifact.type_id()
            )))
        }
    }

    /// Copies the jassets default directory and the j4rs dynamic library under the specified
    /// location.
    /// This is useful for cases when `with_base_path` method is used when building a Jvm with
    /// the JvmBuilder.
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
        let dynlibs: Vec<String> = {
            let mut dynlibs = vec![];
            // We try every 1 second for 10 iterations because on most systems, cargo will
            // parallelize the build and the dynlib might not be created yet.
            for _i in 0..10 {
                dynlibs = utils::find_j4rs_dynamic_libraries_paths()?;
                if dynlibs.is_empty() {
                    thread::sleep(time::Duration::from_millis(1000));
                } else {
                    break;
                }
            }
            dynlibs
        };
        if dynlibs.is_empty() {
            let message = format!(
                "No j4rs dynamic libraries found for target triple {}. \
                                  The host triple during build is {}.",
                env::var("TARGET").unwrap_or("".to_string()),
                env::var("HOST").unwrap_or("UNKNOWN".to_string())
            );
            println!("cargo:warning={}", message);
        }

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
        unsafe {
            let _ = jni_utils::throw_exception(message, self.jni_env)?;
        }
        Ok(())
    }

    pub(crate) fn do_return<T>(jni_env: *mut JNIEnv, to_return: T) -> errors::Result<T> {
        unsafe {
            if (opt_to_res(cache::get_jni_exception_check())?)(jni_env) == JNI_TRUE {
                (opt_to_res(cache::get_jni_exception_describe())?)(jni_env);
                (opt_to_res(cache::get_jni_exception_clear())?)(jni_env);
                Err(J4RsError::JavaError(
                    "An Exception was thrown by Java... Please check the logs or the console."
                        .to_string(),
                ))
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
            tweaks::get_created_java_vms(
                &mut Vec::with_capacity(created_vms_size as usize),
                0,
                &mut created_vms_size,
            );

            if created_vms_size == 0 {
                None
            } else {
                debug(&format!(
                    "Retrieving the first of {} created JVMs",
                    created_vms_size
                ));
                // Get the created VM (use 2 just in case... :) )
                let mut buffer: Vec<*mut JavaVM> = Vec::with_capacity(2);
                for _ in 0..created_vms_size {
                    buffer.push(ptr::null_mut());
                }

                let retjint = tweaks::get_created_java_vms(
                    &mut buffer,
                    created_vms_size,
                    &mut created_vms_size,
                );
                if retjint == JNI_OK {
                    let act = (**buffer[0]).v1_4.AttachCurrentThread;
                    let mut jni_environment: *mut JNIEnv = ptr::null_mut();
                    (act)(
                        buffer[0],
                        (&mut jni_environment as *mut *mut JNIEnv) as *mut *mut c_void,
                        ptr::null_mut(),
                    );
                    Some(jni_environment)
                } else {
                    error(&format!(
                        "Error while retrieving the created JVMs: {}",
                        retjint
                    ));
                    None
                }
            }
        }
    }

    fn detach_current_thread(&self) {
        unsafe {
            // Get the number of the already created VMs. This is most probably 1, but we retrieve the number just in case...
            let mut created_vms_size: jsize = 0;
            tweaks::get_created_java_vms(
                &mut Vec::with_capacity(created_vms_size as usize),
                0,
                &mut created_vms_size,
            );

            if created_vms_size > 0 {
                // Get the created VM
                let mut buffer: Vec<*mut JavaVM> = Vec::with_capacity(created_vms_size as usize);
                for _ in 0..created_vms_size {
                    buffer.push(ptr::null_mut());
                }

                let retjint = tweaks::get_created_java_vms(
                    &mut buffer,
                    created_vms_size,
                    &mut created_vms_size,
                );
                if retjint == JNI_OK {
                    let dct = (**buffer[0]).v1_4.DetachCurrentThread;
                    (dct)(buffer[0]);
                } else {
                    warn(&format!(
                        "Error while retrieving the created JVMs: {}",
                        retjint
                    ));
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
    pub fn select_timeout(
        instance_receivers: &[&InstanceReceiver],
        timeout: &time::Duration,
    ) -> errors::Result<(usize, Instance)> {
        let start = time::Instant::now();
        loop {
            for (index, ir) in instance_receivers.iter().enumerate() {
                let res = ir.rx.try_recv();
                if res.is_ok() {
                    return Ok((index, res.unwrap()));
                }
            }
            if &start.elapsed() > timeout {
                return Err(J4RsError::Timeout);
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
    default_classloader: bool,
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
            default_classloader: false,
        }
    }

    /// Adds a classpath entry.
    pub fn classpath_entry(&'a mut self, cp_entry: ClasspathEntry<'a>) -> &'a mut JvmBuilder {
        self.classpath_entries.push(cp_entry);
        self
    }

    /// Adds classpath entries.
    pub fn classpath_entries(
        &'a mut self,
        cp_entries: Vec<ClasspathEntry<'a>>,
    ) -> &'a mut JvmBuilder {
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

    /// `j4rs` uses a custom ClassLoader (namely the `J4rsClassLoader`),
    /// that allows adding jars to the classpath during runtime, after the underlying `JVM` is initialized.
    ///
    /// This function instructs the builder not to use this custom classloader, but use the default one.
    ///
    /// Please note that the `J4rsClassLoader` needs Java 9 or higher. If you use an older Java version,
    /// you __must__ call this function in order for `j4rs` to work.
    ///
    /// If not, you will get exceptions like the following:
    ///
    /// java.lang.NoSuchMethodError: java.net.URLClassLoader.<init>(Ljava/lang/String;[Ljava/net/URL;Ljava/lang/ClassLoader;)V
    //         at org.astonbitecode.j4rs.api.deploy.J4rsClassLoader.<init>(J4rsClassLoader.java:22)
    //         at sun.reflect.NativeConstructorAccessorImpl.newInstance0(Native Method)
    //         at sun.reflect.NativeConstructorAccessorImpl.newInstance(NativeConstructorAccessorImpl.java:62)
    //         at sun.reflect.DelegatingConstructorAccessorImpl.newInstance(DelegatingConstructorAccessorImpl.java:45)
    //         at java.lang.reflect.Constructor.newInstance(Constructor.java:423)
    //         at java.lang.SystemClassLoaderAction.run(ClassLoader.java:2204)
    //         at java.lang.SystemClassLoaderAction.run(ClassLoader.java:2188)
    //         at java.security.AccessController.doPrivileged(Native Method)
    //         at java.lang.ClassLoader.initSystemClassLoader(ClassLoader.java:1449)
    //         at java.lang.ClassLoader.getSystemClassLoader(ClassLoader.java:1429)
    pub fn with_default_classloader(&'a mut self) -> &'a mut JvmBuilder {
        self.default_classloader = true;
        self
    }

    /// Creates a Jvm
    pub fn build(&mut self) -> errors::Result<Jvm> {
        if !self.default_classloader {
            // Define the system classloader
            self.java_opts.push(JavaOpt::new(
                "-Djava.system.class.loader=org.astonbitecode.j4rs.api.deploy.J4rsClassLoader",
            ));
            self.java_opts.push(JavaOpt::new("-Xshare:off"));
            self.java_opts.push(JavaOpt::new(
                "-Djdk.net.URLClassPath.showIgnoredClassPathEntries=true",
            ));
        }

        let classpath = if self.no_implicit_classpath {
            self.classpath_entries
                .iter()
                .fold(".".to_string(), |all, elem| {
                    format!("{}{}{}", all, utils::classpath_sep(), elem.to_string())
                })
        } else {
            // The default classpath contains all the jars in the jassets directory
            let jassets_path = self.get_jassets_path()?;
            let all_jars = get_dir_content(&jassets_path)?.files;
            // This is the j4rs jar that should be included in the classpath
            let j4rs_jar_to_use = format!("j4rs-{}-jar-with-dependencies.jar", j4rs_version());
            let j4rs_testing_jar_to_use = format!("j4rs-testing-{}.jar", j4rs_version());
            // Filter out possible incorrect jars of j4rs
            let filtered_jars: Vec<String> = all_jars
                .into_iter()
                .filter(|jar_full_path| {
                    let jarname = jar_full_path
                        .split(MAIN_SEPARATOR)
                        .last()
                        .unwrap_or(jar_full_path);
                    !jarname.contains("j4rs-") || jarname.ends_with(&j4rs_jar_to_use) || jarname.ends_with(&j4rs_testing_jar_to_use)
                })
                .collect();
            let cp_string = filtered_jars.join(utils::classpath_sep());

            let default_class_path = format!("-Djava.class.path={}", cp_string);

            self.classpath_entries
                .iter()
                .fold(default_class_path, |all, elem| {
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
            jvm_options.push(
                "--add-modules javafx.base,javafx.controls,javafx.graphics,javafx.fxml".to_string(),
            );
        }
        self.java_opts
            .clone()
            .into_iter()
            .for_each(|opt| jvm_options.push(opt.to_string()));

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
                info(&format!(
                    "Deps directory not found. Setting the library name to search to default: {}",
                    default_lib_name
                ));
                vec![default_lib_name]
            };

            let lib_name_opt = if found_libs.len() > 0 {
                let a_lib = found_libs[0].clone().replace("lib", "");

                let dot_splitted: Vec<&str> = a_lib.split(".").collect();
                let name = dot_splitted[0].to_string();
                info(&format!(
                    "Passing to the Java world the name of the library to load: {}",
                    name
                ));
                Some(name)
            } else {
                None
            };
            lib_name_opt
        } else if self.lib_name_opt.is_some() && !self.skip_setting_native_lib {
            let name = self.lib_name_opt.clone();
            info(&format!(
                "Passing to the Java world the name of the library to load: {}",
                name.as_ref().unwrap()
            ));
            name
        } else {
            None
        };

        provisioning::set_maven_settings(&self.maven_settings);

        Jvm::new(&jvm_options, lib_name_opt).and_then(|mut jvm| {
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

/// Represents default, known Classes in Java. Can be used as class argument in `Jvm#java_list`, etc.
pub enum JavaClass<'a> {
    Void,
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

impl<'a> JavaClass<'a> {
    pub fn get_class_str(&self) -> &'a str {
        match self {
            Self::Void => "void",
            Self::String => CLASS_STRING,
            Self::Boolean => CLASS_BOOLEAN,
            Self::Byte => CLASS_BYTE,
            Self::Character => CLASS_CHARACTER,
            Self::Short => CLASS_SHORT,
            Self::Integer => CLASS_INTEGER,
            Self::Long => CLASS_LONG,
            Self::Float => CLASS_FLOAT,
            Self::Double => CLASS_DOUBLE,
            Self::List => CLASS_LIST,
            Self::Of(str) => str,
        }
    }
}

impl<'a> From<JavaClass<'a>> for &'a str {
    fn from(java_class: JavaClass<'a>) -> &'a str {
        java_class.get_class_str()
    }
}

impl<'a> From<&'a str> for JavaClass<'a> {
    fn from(java_class: &'a str) -> JavaClass<'a> {
        match java_class {
            "void" => Self::Void,
            CLASS_STRING => Self::String,
            CLASS_BOOLEAN => Self::Boolean,
            CLASS_BYTE => Self::Byte,
            CLASS_CHARACTER => Self::Character,
            CLASS_SHORT => Self::Short,
            CLASS_INTEGER => Self::Integer,
            CLASS_LONG => Self::Long,
            CLASS_FLOAT => Self::Float,
            CLASS_DOUBLE => Self::Double,
            CLASS_LIST => Self::List,
            str => Self::Of(str),
        }
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

/// A classpath entry.
#[derive(Debug, Clone)]
pub struct ClasspathEntry<'a>(&'a str);

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
pub struct JavaOpt<'a>(&'a str);

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
    use super::*;

    fn create_tests_jvm() -> errors::Result<Jvm> {
        let jvm: Jvm = JvmBuilder::new().build()?;
        jvm.deploy_artifact(&MavenArtifact::from(format!("io.github.astonbitecode:j4rs-testing:{}", j4rs_version()).as_str()))?;
        Ok(jvm)
    }

    #[test]
    fn jvm_builder() -> errors::Result<()> {
        let res = create_tests_jvm();
        assert!(res.is_ok());
        let one_more_res = JvmBuilder::already_initialized();
        assert!(one_more_res.is_ok());

        Ok(())
    }

    #[test]
    fn test_copy_j4rs_libs_under() -> errors::Result<()> {
        let newdir = "./newdir";
        Jvm::copy_j4rs_libs_under(newdir)?;

        let _ = fs_extra::remove_items(&vec![newdir]);

        Ok(())
    }

    #[test]
    fn test_select() -> errors::Result<()> {
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
        assert_eq!(index1, 2);
        assert_eq!(index2, 0);
        assert_eq!(index3, 2);

        Ok(())
    }

    #[test]
    fn test_select_timeout() -> errors::Result<()> {
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
        let (index1, _) = Jvm::select_timeout(&[&ir1, &ir2], &d)?;
        let (index2, _) = Jvm::select_timeout(&[&ir1, &ir2], &d)?;
        assert!(Jvm::select_timeout(&[&ir1, &ir2], &d).is_err());
        assert_eq!(index1, 0);
        assert_eq!(index2, 1);

        Ok(())
    }

    #[test]
    fn test_java_class_creation() -> errors::Result<()> {
        assert_eq!(JavaClass::Void.get_class_str(), "void");
        assert_eq!(JavaClass::String.get_class_str(), CLASS_STRING);
        assert_eq!(JavaClass::Boolean.get_class_str(), CLASS_BOOLEAN);
        assert_eq!(JavaClass::Byte.get_class_str(), CLASS_BYTE);
        assert_eq!(JavaClass::Character.get_class_str(), CLASS_CHARACTER);
        assert_eq!(JavaClass::Short.get_class_str(), CLASS_SHORT);
        assert_eq!(JavaClass::Integer.get_class_str(), CLASS_INTEGER);
        assert_eq!(JavaClass::Long.get_class_str(), CLASS_LONG);
        assert_eq!(JavaClass::Float.get_class_str(), CLASS_FLOAT);
        assert_eq!(JavaClass::Double.get_class_str(), CLASS_DOUBLE);
        assert_eq!(JavaClass::List.get_class_str(), CLASS_LIST);
        assert_eq!(
            JavaClass::Of("a.java.Class").get_class_str(),
            "a.java.Class"
        );

        Ok(())
    }

    #[test]
    fn test_byte_array_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: Vec<i8> = vec![-3_i8, 7_i8, 8_i8];
        let ia: Vec<_> = rust_value.iter().map(|x| InvocationArg::try_from(x).unwrap().into_primitive().unwrap()).collect();
        let java_instance = jvm.create_java_array(PRIMITIVE_BYTE, &ia)?;
        let rust_value_from_java: Vec<i8> = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_short_array_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: Vec<i16> = vec![-3_i16, 7_i16, 10000_i16];
        let ia: Vec<_> = rust_value.iter().map(|x| InvocationArg::try_from(x).unwrap().into_primitive().unwrap()).collect();
        let java_instance = jvm.create_java_array(PRIMITIVE_SHORT, &ia)?;
        let rust_value_from_java: Vec<i16> = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_char_array_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: Vec<u16> = vec![3_u16, 7_u16, 10000_u16];
        let ia: Vec<_> = rust_value.iter().map(|x| InvocationArg::try_from(x).unwrap().into_primitive().unwrap()).collect();
        let java_instance = jvm.create_java_array(PRIMITIVE_CHAR, &ia)?;
        let rust_value_from_java: Vec<u16> = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_int_array_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: Vec<i32> = vec![-100_000, -1_000_000, 1_000_000];
        let ia: Vec<_> = rust_value.iter().map(|x| InvocationArg::try_from(x).unwrap().into_primitive().unwrap()).collect();
        let java_instance = jvm.create_java_array(PRIMITIVE_INT, &ia)?;
        let rust_value_from_java: Vec<i32> = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_long_array_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: Vec<i64> = vec![3, 7, 8];
        let ia: Vec<_> = rust_value.iter().map(|x| InvocationArg::try_from(x).unwrap().into_primitive().unwrap()).collect();
        let java_instance = jvm.create_java_array(PRIMITIVE_LONG, &ia)?;
        let rust_value_from_java: Vec<i64> = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_float_array_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: Vec<f32> = vec![3_f32, 7_f32, 8_f32];
        let ia: Vec<_> = rust_value.iter().map(|x| InvocationArg::try_from(x).unwrap().into_primitive().unwrap()).collect();
        let java_instance = jvm.create_java_array(PRIMITIVE_FLOAT, &ia)?;
        let rust_value_from_java: Vec<f32> = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_double_array_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: Vec<f64> = vec![3_f64, 7_f64, 8_f64];
        let ia: Vec<_> = rust_value.iter().map(|x| InvocationArg::try_from(x).unwrap().into_primitive().unwrap()).collect();
        let java_instance = jvm.create_java_array(PRIMITIVE_DOUBLE, &ia)?;
        let rust_value_from_java: Vec<f64> = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_int_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: i32 = 3;
        let ia = InvocationArg::try_from(rust_value)?.into_primitive()?;
        let java_instance = jvm.create_instance(CLASS_INTEGER, &[ia])?;
        let java_primitive_instance = jvm.invoke(&java_instance, "intValue", InvocationArg::empty())?;
        let rust_value_from_java: i32 = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);
        let rust_value_from_java: i32 = jvm.to_rust(java_primitive_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_byte_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: i8 = 3;
        let ia = InvocationArg::try_from(rust_value)?.into_primitive()?;
        let java_instance = jvm.create_instance(CLASS_BYTE, &[ia])?;
        let java_primitive_instance = jvm.invoke(&java_instance, "byteValue", InvocationArg::empty())?;
        let rust_value_from_java: i8 = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);
        let rust_value_from_java: i8 = jvm.to_rust(java_primitive_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_short_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: i16 = 3;
        let ia = InvocationArg::try_from(rust_value)?.into_primitive()?;
        let java_instance = jvm.create_instance(CLASS_SHORT, &[ia])?;
        let java_primitive_instance = jvm.invoke(&java_instance, "shortValue", InvocationArg::empty())?;
        let rust_value_from_java: i16 = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);
        let rust_value_from_java: i16 = jvm.to_rust(java_primitive_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_long_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: i64 = 3;
        let ia = InvocationArg::try_from(rust_value)?.into_primitive()?;
        let java_instance = jvm.create_instance(CLASS_LONG, &[ia])?;
        let java_primitive_instance = jvm.invoke(&java_instance, "longValue", InvocationArg::empty())?;
        let rust_value_from_java: i64 = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);
        let rust_value_from_java: i64 = jvm.to_rust(java_primitive_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_float_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: f32 = 3.3;
        let ia = InvocationArg::try_from(rust_value)?.into_primitive()?;
        let java_instance = jvm.create_instance(CLASS_FLOAT, &[ia])?;
        let java_primitive_instance = jvm.invoke(&java_instance, "floatValue", InvocationArg::empty())?;
        let rust_value_from_java: f32 = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);
        let rust_value_from_java: f32 = jvm.to_rust(java_primitive_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn test_double_to_rust() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let rust_value: f64 = 3.3;
        let ia = InvocationArg::try_from(rust_value)?.into_primitive()?;
        let java_instance = jvm.create_instance(CLASS_DOUBLE, &[ia])?;
        let java_primitive_instance = jvm.invoke(&java_instance, "doubleValue", InvocationArg::empty())?;
        let rust_value_from_java: f64 = jvm.to_rust(java_instance)?;
        assert_eq!(rust_value_from_java, rust_value);
        let rust_value_from_java: f64 = jvm.to_rust(java_primitive_instance)?;
        assert_eq!(rust_value_from_java, rust_value);

        Ok(())
    }

    #[test]
    fn api_by_ref_or_value() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        // Instantiate
        let inv_arg1 = InvocationArg::try_from("some string")?;
        let _ = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[&inv_arg1])?;
        let _ = jvm.create_instance("java.lang.String", &[inv_arg1])?;
        Ok(())
    }
}
