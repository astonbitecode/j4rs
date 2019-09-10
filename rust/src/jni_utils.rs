// Copyright 2019 astonbitecode
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

use jni_sys::{jint, JNI_TRUE, JNIEnv, jobject, jobjectRefType};

use crate::{InvocationArg, Jvm};
use crate::api::INVO_IFACE_NAME;
use crate::errors;
use crate::logger::{debug, error};
use crate::utils;

pub(crate) fn invocation_arg_jobject_from_rust_serialized(ia: &InvocationArg, jvm: &Jvm) -> jobject {
    unsafe {
        // The constructor of `InvocationArg` for Rust created args
        let inv_arg_rust_constructor_method = (jvm.jni_get_method_id)(
            jvm.jni_env,
            jvm.invocation_arg_class,
            utils::to_java_string("<init>"),
            utils::to_java_string("(Ljava/lang/String;Ljava/lang/String;)V"));

        let (class_name, json) = match ia {
            _s @ &InvocationArg::Java { .. } | _s @ &InvocationArg::RustBasic { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_serialized for an InvocationArg that contains an object. Please consider opening a bug to the developers.")
            }
            &InvocationArg::Rust { ref class_name, ref json, .. } => {
                debug(&format!("Creating jobject from Rust with serialized representation for class {}", class_name));
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

pub(crate) fn invocation_arg_jobject_from_rust_basic(ia: &InvocationArg, jvm: &Jvm) -> jobject {
    unsafe {
        let signature = "(Ljava/lang/String;Ljava/lang/Object;)V";
        // The constructor of `InvocationArg` for Rust-basic created args (Objects created with JNI)
        let inv_arg_java_constructor_method = (jvm.jni_get_method_id)(
            jvm.jni_env,
            jvm.invocation_arg_class,
            utils::to_java_string("<init>"),
            utils::to_java_string(&signature));

        let (class_name, jinstance) = match ia {
            _s @ &InvocationArg::Java { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_basic for an InvocationArg that contains an object from Java. Please consider opening a bug to the developers.")
            }
            _s @ &InvocationArg::Rust { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_basic for an InvocationArg that contains a serialized object. Please consider opening a bug to the developers.")
            }
            &InvocationArg::RustBasic { ref class_name, ref instance, .. } => {
                debug(&format!("Creating jobject from Rust for class {}", class_name));
                (class_name.to_owned(), instance)
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

pub(crate) fn invocation_arg_jobject_from_java(ia: &InvocationArg, jvm: &Jvm) -> jobject {
    unsafe {
        let signature = format!("(Ljava/lang/String;L{};)V", INVO_IFACE_NAME);
        // The constructor of `InvocationArg` for Java created args
        let inv_arg_java_constructor_method = (jvm.jni_get_method_id)(
            jvm.jni_env,
            jvm.invocation_arg_class,
            utils::to_java_string("<init>"),
            utils::to_java_string(&signature));

        let (class_name, jinstance) = match ia {
            _s @ &InvocationArg::Rust { .. } => panic!("Called invocation_arg_jobject_from_java for an InvocationArg that is created by Rust. Please consider opening a bug to the developers."),
            &InvocationArg::Java { ref class_name, ref instance, .. } | &InvocationArg::RustBasic { ref class_name, ref instance, .. } => {
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

pub(crate) fn _create_weak_global_ref_from_global_ref(global_ref: jobject, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
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
pub(crate) fn delete_java_ref(jni_env: *mut JNIEnv, jinstance: jobject) {
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
pub(crate) fn delete_java_local_ref(jni_env: *mut JNIEnv, jinstance: jobject) {
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

pub(crate) fn jobject_from_str(string: &str, jvm: &Jvm) -> errors::Result<jobject> {
    unsafe {
        let obj = (jvm.jni_new_string_utf)(
            jvm.jni_env,
            utils::to_java_string(string),
        );
        create_global_ref_from_local_ref(obj, jvm.jni_env)
    }
}