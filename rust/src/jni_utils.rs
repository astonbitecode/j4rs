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

use std::os::raw::{c_char, c_double};
use std::ptr;

use jni_sys::{jint, JNI_TRUE, JNIEnv, jobject, jobjectRefType, jstring};

use crate::{InvocationArg, Jvm};
use crate::cache;
use crate::errors;
use crate::errors::opt_to_res;
use crate::logger::{debug, error};
use crate::utils;

pub(crate) fn invocation_arg_jobject_from_rust_serialized(ia: &InvocationArg, jni_env: *mut JNIEnv, create_global: bool) -> errors::Result<jobject> {
    unsafe {
        let (class_name, json) = match ia {
            _s @ &InvocationArg::Java { .. } | _s @ &InvocationArg::RustBasic { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_serialized for an InvocationArg that contains an object. Please consider opening a bug to the developers.")
            }
            &InvocationArg::Rust { ref class_name, ref json, .. } => {
                debug(&format!("Creating jobject from Rust with serialized representation for class {}", class_name));
                (class_name.to_owned(), json.to_owned())
            }
        };

        let class_name_jstring = global_jobject_from_str(&class_name, jni_env)?;
        let json_jstring = global_jobject_from_str(&json, jni_env)?;

        debug(&format!("Calling the InvocationArg constructor with '{}'", class_name));
        let inv_arg_instance = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_invocation_arg_class()?,
            cache::get_inv_arg_rust_constructor_method()?,
            // First argument: class_name
            class_name_jstring,
            // Second argument: json
            json_jstring,
        );

        // Check for exceptions
        Jvm::do_return(jni_env, ())?;
        delete_java_ref(jni_env, class_name_jstring);
        delete_java_ref(jni_env, json_jstring);

        if create_global {
            Ok(create_global_ref_from_local_ref(inv_arg_instance, jni_env)?)
        } else {
            Ok(inv_arg_instance)
        }
    }
}

pub(crate) fn invocation_arg_jobject_from_rust_basic(ia: &InvocationArg, jni_env: *mut JNIEnv, create_global: bool) -> errors::Result<jobject> {
    unsafe {
        let (class_name, jinstance) = match ia {
            _s @ &InvocationArg::Java { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_basic for an InvocationArg that contains an object from Java. Please consider opening a bug to the developers.")
            }
            _s @ &InvocationArg::Rust { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_basic for an InvocationArg that contains a serialized object. Please consider opening a bug to the developers.")
            }
            &InvocationArg::RustBasic { ref class_name, ref instance, .. } => {
                debug(&format!("Creating jobject from Rust basic for class {}", class_name));
                (class_name.to_owned(), instance.jinstance)
            }
        };
        debug(&format!("Calling the InvocationArg constructor with '{}'", class_name));
        let class_name_jstring = global_jobject_from_str(&class_name, jni_env)?;

        let inv_arg_instance = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_invocation_arg_class()?,
            cache::get_inv_arg_basic_rust_constructor_method()?,
            // First argument: class_name
            class_name_jstring,
            // Second argument: Instance instance
            jinstance,
        );

        delete_java_ref(jni_env, class_name_jstring);

        if create_global {
            Ok(create_global_ref_from_local_ref(inv_arg_instance, jni_env)?)
        } else {
            Ok(inv_arg_instance)
        }
    }
}

pub(crate) fn invocation_arg_jobject_from_java(ia: &InvocationArg, jni_env: *mut JNIEnv, create_global: bool) -> errors::Result<jobject> {
    unsafe {
        let (class_name, jinstance) = match ia {
            _s @ &InvocationArg::Rust { .. } => panic!("Called invocation_arg_jobject_from_java for an InvocationArg that is created by Rust. Please consider opening a bug to the developers."),
            &InvocationArg::Java { ref class_name, ref instance, .. } | &InvocationArg::RustBasic { ref class_name, ref instance, .. } => {
                debug(&format!("Creating jobject from Java for class {}", class_name));
                (class_name.to_owned(), instance.jinstance)
            }
        };

        debug(&format!("Calling the InvocationArg constructor for class '{}'", class_name));

        let class_name_jstring = global_jobject_from_str(&class_name, jni_env)?;

        let inv_arg_instance = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_invocation_arg_class()?,
            cache::get_inv_arg_java_constructor_method()?,
            // First argument: class_name
            class_name_jstring,
            // Second argument: Instance instance
            jinstance,
        );

        // Check for exceptions
        Jvm::do_return(jni_env, ())?;
        delete_java_ref(jni_env, class_name_jstring);

        if create_global {
            Ok(create_global_ref_from_local_ref(inv_arg_instance, jni_env)?)
        } else {
            Ok(inv_arg_instance)
        }
    }
}

pub fn create_global_ref_from_local_ref(local_ref: jobject, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        match ((**jni_env).NewGlobalRef,
               (**jni_env).ExceptionCheck,
               (**jni_env).ExceptionDescribe,
               (**jni_env).ExceptionClear,
               (**jni_env).GetObjectRefType) {
            (Some(ngr), Some(exc), Some(exd), Some(exclear), Some(gort)) => {
                // Create the global ref
                let global = ngr(
                    jni_env,
                    local_ref,
                );
                // If local ref, delete it
                if gort(jni_env, local_ref) as jint == jobjectRefType::JNILocalRefType as jint {
                    delete_java_local_ref(jni_env, local_ref);
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
            (_, _, _, _, _) => {
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
pub fn delete_java_ref(jni_env: *mut JNIEnv, jinstance: jobject) {
    unsafe {
        match ((**jni_env).DeleteGlobalRef,
               (**jni_env).ExceptionCheck,
               (**jni_env).ExceptionDescribe,
               (**jni_env).ExceptionClear) {
            (Some(dgr), Some(exc), Some(exd), Some(exclear)) => {
                dgr(
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

pub(crate) fn global_jobject_from_str(string: &str, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        let tmp = utils::to_c_string_struct(string);
        let obj = (opt_to_res(cache::get_jni_new_string_utf())?)(
            jni_env,
            tmp.as_ptr(),
        );
        let gr = create_global_ref_from_local_ref(obj, jni_env)?;
        Ok(gr)
    }
}

pub(crate) fn global_jobject_from_i8(a: &i8, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        let tmp = a.clone() as *const i8;
        let o = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_byte_class()?,
            cache::get_byte_constructor_method()?,
            tmp as *const i8,
        );
        create_global_ref_from_local_ref(o, jni_env)
    }
}

pub(crate) unsafe fn i8_from_jobject(obj: jobject, jni_env: *mut JNIEnv) -> errors::Result<i8> {
    if obj.is_null() {
        Err(errors::J4RsError::JniError("Attempt to create an i8 from null".to_string()))
    } else {
        let v = (opt_to_res(cache::get_jni_call_object_method())?)(
            jni_env,
            obj,
            cache::get_byte_to_byte_method()?,
        );
        Ok(v as i8)
    }
}

pub(crate) fn global_jobject_from_i16(a: &i16, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        let tmp = a.clone() as *const i16;
        let o = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_short_class()?,
            cache::get_short_constructor_method()?,
            tmp as *const i16,
        );
        create_global_ref_from_local_ref(o, jni_env)
    }
}

pub(crate) unsafe fn i16_from_jobject(obj: jobject, jni_env: *mut JNIEnv) -> errors::Result<i16> {
    if obj.is_null() {
        Err(errors::J4RsError::JniError("Attempt to create an i16 from null".to_string()))
    } else {
        let v = (opt_to_res(cache::get_jni_call_object_method())?)(
            jni_env,
            obj,
            cache::get_short_to_short_method()?,
        );
        Ok(v as i16)
    }
}

pub(crate) fn global_jobject_from_i32(a: &i32, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        let tmp = a.clone() as *const i32;
        let o = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_integer_class()?,
            cache::get_integer_constructor_method()?,
            tmp as *const i32,
        );
        create_global_ref_from_local_ref(o, jni_env)
    }
}

pub(crate) unsafe fn i32_from_jobject(obj: jobject, jni_env: *mut JNIEnv) -> errors::Result<i32> {
    if obj.is_null() {
        Err(errors::J4RsError::JniError("Attempt to create an i32 from null".to_string()))
    } else {
        let v = (opt_to_res(cache::get_jni_call_object_method())?)(
            jni_env,
            obj,
            cache::get_integer_to_int_method()?,
        );
        Ok(v as i32)
    }
}

pub(crate) fn global_jobject_from_i64(a: &i64, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    unsafe {
        let tmp = a.clone();
        let o = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_long_class()?,
            cache::get_long_constructor_method()?,
            tmp as *const i64,
        );
        create_global_ref_from_local_ref(o, jni_env)
    }
}

pub(crate) unsafe fn i64_from_jobject(obj: jobject, jni_env: *mut JNIEnv) -> errors::Result<i64> {
    if obj.is_null() {
        Err(errors::J4RsError::JniError("Attempt to create an i64 from null".to_string()))
    } else {
        let v = (opt_to_res(cache::get_jni_call_object_method())?)(
            jni_env,
            obj,
            cache::get_long_to_long_method()?,
        );
        Ok(v as i64)
    }
}

pub(crate) fn global_jobject_from_f32(a: &f32, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    let tmp = a.clone();
    unsafe {
        let o = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_float_class()?,
            cache::get_float_constructor_method()?,
            tmp as c_double,
        );
        create_global_ref_from_local_ref(o, jni_env)
    }
}

pub(crate) unsafe fn f32_from_jobject(obj: jobject, jni_env: *mut JNIEnv) -> errors::Result<f32> {
    if obj.is_null() {
        Err(errors::J4RsError::JniError("Attempt to create an f32 from null".to_string()))
    } else {
        let v = (opt_to_res(cache::get_jni_call_float_method())?)(
            jni_env,
            obj,
            cache::get_float_to_float_method()?,
        );
        Ok(v)
    }
}

pub(crate) fn global_jobject_from_f64(a: &f64, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
    let tmp = a.clone();
    unsafe {
        let o = (opt_to_res(cache::get_jni_new_object())?)(
            jni_env,
            cache::get_double_class()?,
            cache::get_double_constructor_method()?,
            tmp as c_double,
        );
        create_global_ref_from_local_ref(o, jni_env)
    }
}

pub(crate) unsafe fn f64_from_jobject(obj: jobject, jni_env: *mut JNIEnv) -> errors::Result<f64> {
    if obj.is_null() {
        Err(errors::J4RsError::JniError("Attempt to create an f64 from null".to_string()))
    } else {
        let v = (opt_to_res(cache::get_jni_call_double_method())?)(
            jni_env,
            obj,
            cache::get_double_to_double_method()?,
        );
        Ok(v)
    }
}

pub fn jstring_to_rust_string(jvm: &Jvm, java_string: jstring) -> errors::Result<String> {
    unsafe {
        let s = (opt_to_res(cache::get_jni_get_string_utf_chars())?)(
            jvm.jni_env,
            java_string,
            ptr::null_mut(),
        ) as *mut c_char;
        let rust_string = utils::to_rust_string(s);
        (opt_to_res(cache::get_jni_release_string_utf_chars())?)(
            jvm.jni_env,
            java_string,
            s,
        );
        Jvm::do_return(jvm.jni_env, rust_string)
    }
}

pub(crate) fn throw_exception(message: &str, jni_env: *mut JNIEnv) -> errors::Result<i32> {
    unsafe {
        let message_jstring = utils::to_c_string_struct(message);
        let i = (opt_to_res(cache::get_jni_throw_new())?)(
            jni_env,
            cache::get_invocation_exception_class()?,
            message_jstring.as_ptr(),
        );
        Ok(i)
    }
}

pub(crate) fn is_same_object(obj1: jobject, obj2: jobject, jni_env: *mut JNIEnv) -> errors::Result<bool> {
    unsafe {
        let b = (opt_to_res(cache::get_is_same_object())?)(
            jni_env,
            obj1,
            obj2,
        );
        Ok(b == JNI_TRUE)
    }
}