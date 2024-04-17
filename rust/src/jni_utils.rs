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

use std::any::TypeId;
use std::os::raw::{c_char, c_double};
use std::ptr;

use jni_sys::{jint, jobject, jobjectRefType, jstring, JNIEnv, JNI_TRUE};

use crate::cache;
use crate::errors;
use crate::errors::opt_to_res;
use crate::logger::{debug, error};
use crate::utils;
use crate::{InvocationArg, Jvm};

pub(crate) fn invocation_arg_jobject_from_rust_serialized(
    ia: &InvocationArg,
    jni_env: *mut JNIEnv,
    create_global: bool,
) -> errors::Result<jobject> {
    unsafe {
        let (class_name, json) = match ia {
            _s @ &InvocationArg::Java { .. } | _s @ &InvocationArg::RustBasic { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_serialized for an InvocationArg that contains an object. Please consider opening a bug to the developers.")
            }
            &InvocationArg::Rust {
                ref class_name,
                ref json,
                ..
            } => {
                debug(&format!(
                    "Creating jobject from Rust with serialized representation for class {}",
                    class_name
                ));
                (class_name.to_owned(), json.to_owned())
            }
        };

        let class_name_jstring = global_jobject_from_str(&class_name, jni_env)?;
        let json_jstring = global_jobject_from_str(&json, jni_env)?;

        debug(&format!(
            "Calling the InvocationArg constructor with '{}'",
            class_name
        ));
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

pub(crate) fn invocation_arg_jobject_from_rust_basic(
    ia: &InvocationArg,
    jni_env: *mut JNIEnv,
    create_global: bool,
) -> errors::Result<jobject> {
    unsafe {
        let (class_name, jinstance) = match ia {
            _s @ &InvocationArg::Java { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_basic for an InvocationArg that contains an object from Java. Please consider opening a bug to the developers.")
            }
            _s @ &InvocationArg::Rust { .. } => {
                panic!("Called invocation_arg_jobject_from_rust_basic for an InvocationArg that contains a serialized object. Please consider opening a bug to the developers.")
            }
            &InvocationArg::RustBasic {
                ref class_name,
                ref instance,
                ..
            } => {
                debug(&format!(
                    "Creating jobject from Rust basic for class {}",
                    class_name
                ));
                (class_name.to_owned(), instance.jinstance)
            }
        };
        debug(&format!(
            "Calling the InvocationArg constructor with '{}'",
            class_name
        ));
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

pub(crate) fn invocation_arg_jobject_from_java(
    ia: &InvocationArg,
    jni_env: *mut JNIEnv,
    create_global: bool,
) -> errors::Result<jobject> {
    unsafe {
        let (class_name, jinstance) = match ia {
            _s @ &InvocationArg::Rust { .. } => panic!("Called invocation_arg_jobject_from_java for an InvocationArg that is created by Rust. Please consider opening a bug to the developers."),
            &InvocationArg::Java { ref class_name, ref instance, .. } | &InvocationArg::RustBasic { ref class_name, ref instance, .. } => {
                debug(&format!("Creating jobject from Java for class {}", class_name));
                (class_name.to_owned(), instance.jinstance)
            }
        };

        debug(&format!(
            "Calling the InvocationArg constructor for class '{}'",
            class_name
        ));

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

pub fn create_global_ref_from_local_ref(
    local_ref: jobject,
    jni_env: *mut JNIEnv,
) -> errors::Result<jobject> {
    unsafe {
        let ngr = (**jni_env).v1_6.NewGlobalRef;
        let exc = (**jni_env).v1_6.ExceptionCheck;
        let exd = (**jni_env).v1_6.ExceptionDescribe;
        let exclear = (**jni_env).v1_6.ExceptionClear;
        let gort = (**jni_env).v1_6.GetObjectRefType;
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
}

pub(crate) fn _create_weak_global_ref_from_global_ref(
    global_ref: jobject,
    jni_env: *mut JNIEnv,
) -> errors::Result<jobject> {
    unsafe {
        let nwgr = (**jni_env).v1_6.NewWeakGlobalRef;
        let exc = (**jni_env).v1_6.ExceptionCheck;
        let exd = (**jni_env).v1_6.ExceptionDescribe;
        let exclear = (**jni_env).v1_6.ExceptionClear;

        // Create the weak global ref
        let global = nwgr(jni_env, global_ref);
        // Exception check
        if (exc)(jni_env) == JNI_TRUE {
            (exd)(jni_env);
            (exclear)(jni_env);
            Err(errors::J4RsError::JavaError("An Exception was thrown by Java while creating a weak global ref... Please check the logs or the console.".to_string()))
        } else {
            Ok(global)
        }
    }
}

/// Deletes the java ref from the memory
pub fn delete_java_ref(jni_env: *mut JNIEnv, jinstance: jobject) {
    unsafe {
        let dgr = (**jni_env).v1_6.DeleteGlobalRef;
        let exc = (**jni_env).v1_6.ExceptionCheck;
        let exd = (**jni_env).v1_6.ExceptionDescribe;
        let exclear = (**jni_env).v1_6.ExceptionClear;
        dgr(jni_env, jinstance);
        if (exc)(jni_env) == JNI_TRUE {
            (exd)(jni_env);
            (exclear)(jni_env);
            error(
                "An Exception was thrown by Java... Please check the logs or the console.",
            );
        }
    }
}

/// Deletes the java ref from the memory
pub(crate) fn delete_java_local_ref(jni_env: *mut JNIEnv, jinstance: jobject) {
    unsafe {
        let dlr = (**jni_env).v1_6.DeleteLocalRef;
        let exc = (**jni_env).v1_6.ExceptionCheck;
        let exd = (**jni_env).v1_6.ExceptionDescribe;
        let exclear = (**jni_env).v1_6.ExceptionClear;
        dlr(jni_env, jinstance);
        if (exc)(jni_env) == JNI_TRUE {
            (exd)(jni_env);
            (exclear)(jni_env);
            error(
                "An Exception was thrown by Java... Please check the logs or the console.",
            );
        }
    }
}

pub(crate) fn global_jobject_from_str(
    string: &str,
    jni_env: *mut JNIEnv,
) -> errors::Result<jobject> {
    unsafe {
        let tmp = utils::to_c_string_struct(string);
        let obj = (opt_to_res(cache::get_jni_new_string_utf())?)(jni_env, tmp.as_ptr());
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
        Err(errors::J4RsError::JniError(
            "Attempt to create an i8 from null".to_string(),
        ))
    } else {
        let v = (opt_to_res(cache::get_jni_call_byte_method())?)(
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
        Err(errors::J4RsError::JniError(
            "Attempt to create an i16 from null".to_string(),
        ))
    } else {
        let v = (opt_to_res(cache::get_jni_call_short_method())?)(
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
        Err(errors::J4RsError::JniError(
            "Attempt to create an i32 from null".to_string(),
        ))
    } else {
        let v = (opt_to_res(cache::get_jni_call_int_method())?)(
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
        Err(errors::J4RsError::JniError(
            "Attempt to create an i64 from null".to_string(),
        ))
    } else {
        let v = (opt_to_res(cache::get_jni_call_long_method())?)(
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
        Err(errors::J4RsError::JniError(
            "Attempt to create an f32 from null".to_string(),
        ))
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
        Err(errors::J4RsError::JniError(
            "Attempt to create an f64 from null".to_string(),
        ))
    } else {
        let v = (opt_to_res(cache::get_jni_call_double_method())?)(
            jni_env,
            obj,
            cache::get_double_to_double_method()?,
        );
        Ok(v)
    }
}

macro_rules! primitive_array_from_jobject {
    ($fn_name:ident, $rust_type:ty, $get_array_element:path, $release_array_element:path) => {
        pub(crate) unsafe fn $fn_name(obj: jobject, jni_env: *mut JNIEnv) -> errors::Result<Vec<$rust_type>> {
            if obj.is_null() {
                Err(errors::J4RsError::JniError(
                    format!("Attempt to create an {:?} array from null", TypeId::of::<$rust_type>()),
                ))
            } else {
                let length = (opt_to_res(cache::get_jni_get_array_length())?)(
                    jni_env,
                    obj
                );
                let bytes = (opt_to_res($get_array_element())?)(
                    jni_env,
                    obj,
                    ptr::null_mut()
                );
                if bytes.is_null() { return Err(errors::J4RsError::JniError("get_<primitive>_array_elements failed".to_string())) }
                let parts_mut = std::slice::from_raw_parts_mut(bytes as *mut $rust_type, length as usize);
                let mut vec = Vec::with_capacity(length as usize);
                vec.extend_from_slice(parts_mut);
                (opt_to_res($release_array_element())?)(
                    jni_env,
                    obj,
                    bytes,
                    jni_sys::JNI_ABORT,
                );
                Ok(vec)
            }
        }
    };
}

primitive_array_from_jobject!(i8_array_from_jobject, i8, cache::get_jni_get_byte_array_elements, cache::get_jni_release_byte_array_elements);
primitive_array_from_jobject!(i16_array_from_jobject, i16, cache::get_jni_get_short_array_elements, cache::get_jni_release_short_array_elements);
primitive_array_from_jobject!(u16_array_from_jobject, u16, cache::get_jni_get_char_array_elements, cache::get_jni_release_char_array_elements);
primitive_array_from_jobject!(i32_array_from_jobject, i32, cache::get_jni_get_int_array_elements, cache::get_jni_release_int_array_elements);
primitive_array_from_jobject!(i64_array_from_jobject, i64, cache::get_jni_get_long_array_elements, cache::get_jni_release_long_array_elements);
primitive_array_from_jobject!(f32_array_from_jobject, f32, cache::get_jni_get_float_array_elements, cache::get_jni_release_float_array_elements);
primitive_array_from_jobject!(f64_array_from_jobject, f64, cache::get_jni_get_double_array_elements, cache::get_jni_release_double_array_elements);
primitive_array_from_jobject!(boolean_array_from_jobject, bool, cache::get_jni_get_boolean_array_elements, cache::get_jni_release_boolean_array_elements);

pub(crate) unsafe fn string_from_jobject(
    obj: jobject,
    jni_env: *mut JNIEnv,
) -> errors::Result<String> {
    if obj.is_null() {
        Err(errors::J4RsError::JniError(
            "Attempt to create a String from null".to_string(),
        ))
    } else {
        let s = (opt_to_res(cache::get_jni_get_string_utf_chars())?)(jni_env, obj, ptr::null_mut())
            as *mut c_char;
        let rust_string = utils::to_rust_string(s);

        Ok(rust_string)
    }
}

pub unsafe fn jstring_to_rust_string(jvm: &Jvm, java_string: jstring) -> errors::Result<String> {
    let s = (opt_to_res(cache::get_jni_get_string_utf_chars())?)(
        jvm.jni_env,
        java_string,
        ptr::null_mut(),
    ) as *mut c_char;
    let rust_string = utils::to_rust_string(s);
    (opt_to_res(cache::get_jni_release_string_utf_chars())?)(jvm.jni_env, java_string, s);
    Jvm::do_return(jvm.jni_env, rust_string)
}

pub(crate) unsafe fn throw_exception(message: &str, jni_env: *mut JNIEnv) -> errors::Result<i32> {
    let message_jstring = utils::to_c_string_struct(message);
    let i = (opt_to_res(cache::get_jni_throw_new())?)(
        jni_env,
        cache::get_invocation_exception_class()?,
        message_jstring.as_ptr(),
    );
    Ok(i)
}
