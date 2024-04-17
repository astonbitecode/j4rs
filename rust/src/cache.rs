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

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Mutex;

use jni_sys::{self, jarray, jboolean, jbooleanArray, jbyte, jbyteArray, jchar, jcharArray, jclass,
              jdouble, jdoubleArray, jfloat, jfloatArray, jint, jintArray, jlong, jlongArray,
              jmethodID, JNIEnv, jobject, jobjectArray, jshort, jshortArray, jsize, jstring};
use libc::c_char;

use crate::errors::opt_to_res;
use crate::logger::debug;
use crate::{api_tweaks as tweaks, errors, jni_utils, utils};

pub(crate) const INST_CLASS_NAME: &'static str =
    "org/astonbitecode/j4rs/api/instantiation/NativeInstantiationImpl";
pub(crate) const INVO_BASE_NAME: &'static str = "org/astonbitecode/j4rs/api/InstanceBase";
pub(crate) const INVO_IFACE_NAME: &'static str = "org/astonbitecode/j4rs/api/Instance";
pub(crate) const UNKNOWN_FOR_RUST: &'static str = "known_in_java_world";
pub(crate) const J4RS_ARRAY: &'static str = "org.astonbitecode.j4rs.api.dtos.Array";

pub(crate) type JniGetMethodId = unsafe extern "system" fn(
    *mut jni_sys::JNIEnv,
    *mut jni_sys::_jobject,
    *const c_char,
    *const c_char,
) -> *mut jni_sys::_jmethodID;
pub(crate) type JniGetStaticMethodId = unsafe extern "system" fn(
    *mut jni_sys::JNIEnv,
    *mut jni_sys::_jobject,
    *const c_char,
    *const c_char,
) -> *mut jni_sys::_jmethodID;
#[allow(non_snake_case)]
pub(crate) type JniNewObject =
    unsafe extern "C" fn(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, ...) -> jobject;
pub(crate) type JniNewStringUTF =
    unsafe extern "system" fn(env: *mut JNIEnv, utf: *const c_char) -> jstring;
#[allow(non_snake_case)]
pub(crate) type JniGetStringUTFChars = unsafe extern "system" fn(
    env: *mut JNIEnv,
    str: jstring,
    isCopy: *mut jboolean,
) -> *const c_char;
#[allow(non_snake_case)]
pub(crate) type JniReleaseStringUTFChars =
    unsafe extern "system" fn(env: *mut JNIEnv, str: jstring, utf: *const c_char);
#[allow(non_snake_case)]
pub(crate) type JniCallObjectMethod =
    unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
#[allow(non_snake_case)]
pub(crate) type JniCallIntMethod =
    unsafe extern "C" fn(_: *mut JNIEnv, _: jobject, _: jmethodID, ...) -> jint;
#[allow(non_snake_case)]
pub(crate) type JniCallByteMethod =
    unsafe extern "C" fn(_: *mut JNIEnv, _: jobject, _: jmethodID, ...) -> jbyte;
#[allow(non_snake_case)]
pub(crate) type JniCallShortMethod =
    unsafe extern "C" fn(_: *mut JNIEnv, _: jobject, _: jmethodID, ...) -> jshort;
#[allow(non_snake_case)]
pub(crate) type JniCallCharMethod =
    unsafe extern "C" fn(_: *mut JNIEnv, _: jobject, _: jmethodID, ...) -> jchar;
#[allow(non_snake_case)]
pub(crate) type JniCallLongMethod =
    unsafe extern "C" fn(_: *mut JNIEnv, _: jobject, _: jmethodID, ...) -> jlong;
#[allow(non_snake_case)]
pub(crate) type JniCallFloatMethod =
    unsafe extern "C" fn(_: *mut JNIEnv, _: jobject, _: jmethodID, ...) -> jfloat;
#[allow(non_snake_case)]
pub(crate) type JniCallDoubleMethod =
    unsafe extern "C" fn(_: *mut JNIEnv, _: jobject, _: jmethodID, ...) -> jdouble;
#[allow(non_snake_case)]
pub(crate) type JniCallVoidMethod =
    unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...);
#[allow(non_snake_case)]
pub(crate) type JniCallStaticObjectMethod =
    unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
pub(crate) type JniGetArrayLength =
    unsafe extern "system" fn(env: *mut JNIEnv, array: jarray) -> jsize;

macro_rules! primitive_array_definitions {
    (
        $jni_get_array_elements_type:ident,
        $jni_release_array_elements_type:ident,
        $jni_get_array_elements_cell:ident,
        $jni_release_array_elements_cell:ident,
        $set_jni_get_array_elements_cell:ident,
        $get_jni_get_array_elements_cell:ident,
        $set_jni_release_array_elements_cell:ident,
        $get_jni_release_array_elements_cell:ident,
        $jarray_type:ty,
        $jtype:ty
    ) => {
        #[allow(non_snake_case)]
        pub(crate) type $jni_get_array_elements_type =
            unsafe extern "system" fn(env: *mut JNIEnv, array: $jarray_type, isCopy: *mut jboolean) -> *mut $jtype;
        #[allow(non_snake_case)]
        pub(crate) type $jni_release_array_elements_type =
            unsafe extern "system" fn(env: *mut JNIEnv, array: $jarray_type, elems: *mut $jtype, mode: jint);

        thread_local! {
            pub(crate) static $jni_get_array_elements_cell: RefCell<Option<$jni_get_array_elements_type>> = RefCell::new(None);
            pub(crate) static $jni_release_array_elements_cell: RefCell<Option<$jni_release_array_elements_type>> = RefCell::new(None);
        }

        pub(crate) fn $set_jni_get_array_elements_cell(
            j: Option<$jni_get_array_elements_type>,
        ) -> Option<$jni_get_array_elements_type> {
            debug(&format!("Called {}", stringify!($set_jni_get_array_elements_cell)));
            $jni_get_array_elements_cell.with(|opt| {
                *opt.borrow_mut() = j;
            });
            $get_jni_get_array_elements_cell()
        }

        pub(crate) fn $get_jni_get_array_elements_cell() -> Option<$jni_get_array_elements_type> {
            $jni_get_array_elements_cell.with(|opt| *opt.borrow())
        }

        pub(crate) fn $set_jni_release_array_elements_cell(
            j: Option<$jni_release_array_elements_type>,
        ) -> Option<$jni_release_array_elements_type> {
            debug(&format!("Called {}", stringify!($set_jni_release_array_elements_cell)));
            $jni_release_array_elements_cell.with(|opt| {
                *opt.borrow_mut() = j;
            });
            $get_jni_release_array_elements_cell()
        }

        pub(crate) fn $get_jni_release_array_elements_cell() -> Option<$jni_release_array_elements_type> {
            $jni_release_array_elements_cell.with(|opt| *opt.borrow())
        }
    };
}

primitive_array_definitions!(JniGetByteArrayElements, JniReleaseByteArrayElements,
    JNI_GET_BYTE_ARRAY_ELEMENTS, JNI_RELEASE_BYTE_ARRAY_ELEMENTS,
    set_jni_get_byte_array_elements, get_jni_get_byte_array_elements,
    set_jni_release_byte_array_elements, get_jni_release_byte_array_elements,
    jbyteArray, jbyte);
primitive_array_definitions!(JniGetShortArrayElements, JniReleaseShortArrayElements,
    JNI_GET_SHORT_ARRAY_ELEMENTS, JNI_RELEASE_SHORT_ARRAY_ELEMENTS,
    set_jni_get_short_array_elements, get_jni_get_short_array_elements,
    set_jni_release_short_array_elements, get_jni_release_short_array_elements,
    jshortArray, jshort);
primitive_array_definitions!(JniGetIntArrayElements, JniReleaseIntArrayElements,
    JNI_GET_INT_ARRAY_ELEMENTS, JNI_RELEASE_INT_ARRAY_ELEMENTS,
    set_jni_get_int_array_elements, get_jni_get_int_array_elements,
    set_jni_release_int_array_elements, get_jni_release_int_array_elements,
    jintArray, jint);
primitive_array_definitions!(JniGetLongArrayElements, JniReleaseLongArrayElements,
    JNI_GET_LONG_ARRAY_ELEMENTS, JNI_RELEASE_LONG_ARRAY_ELEMENTS,
    set_jni_get_long_array_elements, get_jni_get_long_array_elements,
    set_jni_release_long_array_elements, get_jni_release_long_array_elements,
    jlongArray, jlong);
primitive_array_definitions!(JniGetFloatArrayElements, JniReleaseFloatArrayElements,
    JNI_GET_FLOAT_ARRAY_ELEMENTS, JNI_RELEASE_FLOAT_ARRAY_ELEMENTS,
    set_jni_get_float_array_elements, get_jni_get_float_array_elements,
    set_jni_release_float_array_elements, get_jni_release_float_array_elements,
    jfloatArray, jfloat);
primitive_array_definitions!(JniGetDoubleArrayElements, JniReleaseDoubleArrayElements,
    JNI_GET_DOUBLE_ARRAY_ELEMENTS, JNI_RELEASE_DOUBLE_ARRAY_ELEMENTS,
    set_jni_get_double_array_elements, get_jni_get_double_array_elements,
    set_jni_release_double_array_elements, get_jni_release_double_array_elements,
    jdoubleArray, jdouble);
primitive_array_definitions!(JniGetCharArrayElements, JniReleaseCharArrayElements,
    JNI_GET_CHAR_ARRAY_ELEMENTS, JNI_RELEASE_CHAR_ARRAY_ELEMENTS,
    set_jni_get_char_array_elements, get_jni_get_char_array_elements,
    set_jni_release_char_array_elements, get_jni_release_char_array_elements,
    jcharArray, jchar);
primitive_array_definitions!(JniGetBooleanArrayElements, JniReleaseBooleanArrayElements,
    JNI_GET_BOOLEAN_ARRAY_ELEMENTS, JNI_RELEASE_BOOLEAN_ARRAY_ELEMENTS,
    set_jni_get_boolean_array_elements, get_jni_get_boolean_array_elements,
    set_jni_release_boolean_array_elements, get_jni_release_boolean_array_elements,
    jbooleanArray, jboolean);

pub(crate) type JniNewObjectArray = unsafe extern "system" fn(
    env: *mut JNIEnv,
    len: jsize,
    clazz: jclass,
    init: jobject,
) -> jobjectArray;
pub(crate) type JniSetObjectArrayElement = unsafe extern "system" fn(
    *mut jni_sys::JNIEnv,
    *mut jni_sys::_jobject,
    i32,
    *mut jni_sys::_jobject,
);
pub(crate) type JniExceptionCheck = unsafe extern "system" fn(_: *mut JNIEnv) -> jboolean;
pub(crate) type JniExceptionDescribe = unsafe extern "system" fn(_: *mut JNIEnv);
pub(crate) type JniExceptionClear = unsafe extern "system" fn(_: *mut JNIEnv);
pub(crate) type JniDeleteLocalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> ();
pub(crate) type JniDeleteGlobalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> ();
pub(crate) type JniNewGlobalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> jobject;
pub(crate) type JniThrowNew =
    unsafe extern "system" fn(_: *mut JNIEnv, _: jclass, _: *const c_char) -> jint;
pub(crate) type JniIsSameObject =
    unsafe extern "system" fn(_: *mut JNIEnv, _: jobject, _: jobject) -> jboolean;

const CLASS_CACHING_ENABLED: bool = !(cfg!(target_os = "android"));

lazy_static! {
    // Synchronize the creation of Jvm
    pub(crate) static ref MUTEX: Mutex<bool> = Mutex::new(false);
    // If a Jvm is created with defining a jassets_path other than the default, this is set here
    pub(crate) static ref JASSETS_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

thread_local! {
    pub(crate) static JNI_ENV: RefCell<Option<*mut JNIEnv>> = RefCell::new(None);
    pub(crate) static ACTIVE_JVMS: RefCell<i32> = RefCell::new(0);
    pub(crate) static JNI_GET_METHOD_ID: RefCell<Option<JniGetMethodId>> = RefCell::new(None);
    pub(crate) static JNI_GET_STATIC_METHOD_ID: RefCell<Option<JniGetStaticMethodId>> = RefCell::new(None);
    pub(crate) static JNI_NEW_OBJECT: RefCell<Option<JniNewObject>> = RefCell::new(None);
    pub(crate) static JNI_NEW_STRING_UTF: RefCell<Option<JniNewStringUTF>> = RefCell::new(None);
    pub(crate) static JNI_GET_STRING_UTF_CHARS: RefCell<Option<JniGetStringUTFChars>> = RefCell::new(None);
    pub(crate) static JNI_RELEASE_STRING_UTF_CHARS: RefCell<Option<JniReleaseStringUTFChars>> = RefCell::new(None);
    pub(crate) static JNI_CALL_OBJECT_METHOD: RefCell<Option<JniCallObjectMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_INT_METHOD: RefCell<Option<JniCallIntMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_BYTE_METHOD: RefCell<Option<JniCallByteMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_SHORT_METHOD: RefCell<Option<JniCallShortMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_CHAR_METHOD: RefCell<Option<JniCallCharMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_LONG_METHOD: RefCell<Option<JniCallLongMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_FLOAT_METHOD: RefCell<Option<JniCallFloatMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_DOUBLE_METHOD: RefCell<Option<JniCallDoubleMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_VOID_METHOD: RefCell<Option<JniCallVoidMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_STATIC_OBJECT_METHOD: RefCell<Option<JniCallStaticObjectMethod>> = RefCell::new(None);
    pub(crate) static JNI_GET_ARRAY_LENGTH: RefCell<Option<JniGetArrayLength>> = RefCell::new(None);
    pub(crate) static JNI_NEW_OBJECT_ARRAY: RefCell<Option<JniNewObjectArray>> = RefCell::new(None);
    pub(crate) static JNI_SET_OBJECT_ARRAY_ELEMENT: RefCell<Option<JniSetObjectArrayElement>> = RefCell::new(None);
    pub(crate) static JNI_EXCEPTION_CHECK: RefCell<Option<JniExceptionCheck>> = RefCell::new(None);
    pub(crate) static JNI_EXCEPTION_DESCRIBE: RefCell<Option<JniExceptionDescribe>> = RefCell::new(None);
    pub(crate) static JNI_EXCEPTION_CLEAR: RefCell<Option<JniExceptionClear>> = RefCell::new(None);
    pub(crate) static JNI_DELETE_LOCAL_REF: RefCell<Option<JniDeleteLocalRef>> = RefCell::new(None);
    pub(crate) static JNI_DELETE_GLOBAL_REF: RefCell<Option<JniDeleteGlobalRef>> = RefCell::new(None);
    pub(crate) static JNI_NEW_GLOBAL_REF: RefCell<Option<JniNewGlobalRef>> = RefCell::new(None);
    pub(crate) static JNI_THROW_NEW: RefCell<Option<JniThrowNew>> = RefCell::new(None);
    pub(crate) static JNI_IS_SAME_OBJECT: RefCell<Option<JniIsSameObject>> = RefCell::new(None);
    // This is the factory class. It creates instances using reflection. Currently the `NativeInstantiationImpl`.
    pub(crate) static FACTORY_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    // The constructor method of the `NativeInstantiationImpl`.
    pub(crate) static FACTORY_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The method id of the `instantiate` method of the `NativeInstantiation`.
    pub(crate) static FACTORY_INSTANTIATE_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The method id of the `createForStatic` method of the `NativeInstantiation`.
    pub(crate) static FACTORY_CREATE_FOR_STATIC_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The method id of the `createJavaArray` method of the `NativeInstantiation`.
    pub(crate) static FACTORY_CREATE_JAVA_ARRAY_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The method id of the `createJavaList` method of the `NativeInstantiation`.
    pub(crate) static FACTORY_CREATE_JAVA_LIST_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The method id of the `createJavaMap` method of the `NativeInstantiation`.
    pub(crate) static FACTORY_CREATE_JAVA_MAP_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The `Instance` class.
    // This is optional because it exists only in Android for Java7 compatibility
    // because Java7 does not support static method implementations in interfaces.
    pub(crate) static JAVA_INSTANCE_BASE_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    // The `Instance` class.
    pub(crate) static JAVA_INSTANCE_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    // The Java class for the `InvocationArg`.
    pub(crate) static INVOCATION_ARG_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    // The invoke method
    pub(crate) static INVOKE_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invoke static method
    pub(crate) static INVOKE_STATIC_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invoke to channel method
    pub(crate) static INVOKE_TO_CHANNEL_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The method that invokes a Java method that returns Future
    pub(crate) static INVOKE_ASYNC_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The init callback channel method
    pub(crate) static INIT_CALLBACK_CHANNEL_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The field method
    pub(crate) static FIELD_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static CLASS_TO_INVOKE_CLONE_AND_CAST: RefCell<Option<jclass>> = RefCell::new(None);
    // The clone method
    pub(crate) static CLONE_STATIC_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The cast method
    pub(crate) static CAST_STATIC_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The get json method
    pub(crate) static GET_JSON_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The get object class name method
    pub(crate) static GET_OBJECT_CLASS_NAME_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The get object method
    pub(crate) static GET_OBJECT_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invstatic ocation argument constructor method for objects created by Java
    pub(crate) static INV_ARG_JAVA_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invstatic ocation argument constructor method for objects created by Rust
    pub(crate) static INV_ARG_RUST_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invstatic ocation argument constructor method for objects of Basic type created by Rust
    pub(crate) static INV_ARG_BASIC_RUST_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // Basic types definitions
    pub(crate) static INTEGER_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static INTEGER_TO_INT_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static INTEGER_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static LONG_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static LONG_TO_LONG_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static LONG_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static SHORT_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static SHORT_TO_SHORT_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static SHORT_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static CHARACTER_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static CHARACTER_TO_CHAR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static CHARACTER_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static BYTE_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static BYTE_TO_BYTE_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static BYTE_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static FLOAT_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static FLOAT_TO_FLOAT_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static FLOAT_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static DOUBLE_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static DOUBLE_TO_DOUBLE_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static DOUBLE_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static INVOCATION_EXCEPTION_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static STRING_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
}

macro_rules! get_cached {
    ($opt_name:ident, $do_retrieve:expr, $setter_name:ident) => {{
        let jopt = if CLASS_CACHING_ENABLED {
            $opt_name.with(|opt| *opt.borrow())
        } else {
            None
        };
        if jopt.is_none() {
            let j = { $do_retrieve };
            if CLASS_CACHING_ENABLED {
                $setter_name(j);
            }
            Ok(j)
        } else {
            Ok(jopt.unwrap())
        }
    }};
}

pub(crate) fn add_active_jvm() {
    ACTIVE_JVMS.with(|active_jvms| {
        let active_number = { *active_jvms.borrow() + 1 };
        *active_jvms.borrow_mut() = active_number;
    });
}

pub(crate) fn remove_active_jvm() -> i32 {
    ACTIVE_JVMS.with(|active_jvms| {
        let active_number = { *active_jvms.borrow() - 1 };
        *active_jvms.borrow_mut() = active_number;
        active_number
    })
}

pub(crate) fn get_thread_local_env_opt() -> Option<*mut JNIEnv> {
    JNI_ENV.with(
        |existing_jni_env_opt| match *existing_jni_env_opt.borrow() {
            Some(env) => Some(env),
            None => None,
        },
    )
}

pub(crate) fn set_thread_local_env(jni_env_opt: Option<*mut JNIEnv>) {
    debug("Called set_thread_local_env");
    JNI_ENV.with(|existing_jni_env_opt| {
        *existing_jni_env_opt.borrow_mut() = jni_env_opt;
    });
}

pub(crate) fn get_thread_local_env() -> errors::Result<*mut JNIEnv> {
    match get_thread_local_env_opt() {
        Some(env) => Ok(env),
        None => Err(errors::J4RsError::JavaError(format!(
            "Could not find the JNIEnv in the thread local"
        ))),
    }
}

pub(crate) fn set_jni_get_method_id(j: Option<JniGetMethodId>) -> Option<JniGetMethodId> {
    debug("Called set_jni_get_method_id");
    JNI_GET_METHOD_ID.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_get_method_id()
}

pub(crate) fn get_jni_get_method_id() -> Option<JniGetMethodId> {
    JNI_GET_METHOD_ID.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_get_static_method_id(
    j: Option<JniGetMethodId>,
) -> Option<JniGetStaticMethodId> {
    debug("Called set_jni_get_static_method_id");
    JNI_GET_STATIC_METHOD_ID.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_get_static_method_id()
}

pub(crate) fn get_jni_get_static_method_id() -> Option<JniGetStaticMethodId> {
    JNI_GET_STATIC_METHOD_ID.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_new_object(j: Option<JniNewObject>) -> Option<JniNewObject> {
    debug("Called set_jni_new_object");
    JNI_NEW_OBJECT.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_object()
}

pub(crate) fn get_jni_new_object() -> Option<JniNewObject> {
    JNI_NEW_OBJECT.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_new_string_utf(j: Option<JniNewStringUTF>) -> Option<JniNewStringUTF> {
    debug("Called set_jni_new_string_utf");
    JNI_NEW_STRING_UTF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_string_utf()
}

pub(crate) fn get_jni_new_string_utf() -> Option<JniNewStringUTF> {
    JNI_NEW_STRING_UTF.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_get_string_utf_chars(
    j: Option<JniGetStringUTFChars>,
) -> Option<JniGetStringUTFChars> {
    debug("Called set_jni_get_string_utf_chars");
    JNI_GET_STRING_UTF_CHARS.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_get_string_utf_chars()
}

pub(crate) fn get_jni_get_string_utf_chars() -> Option<JniGetStringUTFChars> {
    JNI_GET_STRING_UTF_CHARS.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_release_string_utf_chars(
    j: Option<JniReleaseStringUTFChars>,
) -> Option<JniReleaseStringUTFChars> {
    debug("Called set_jni_release_string_utf_chars");
    JNI_RELEASE_STRING_UTF_CHARS.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_release_string_utf_chars()
}

pub(crate) fn get_jni_release_string_utf_chars() -> Option<JniReleaseStringUTFChars> {
    JNI_RELEASE_STRING_UTF_CHARS.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_object_method(
    j: Option<JniCallObjectMethod>,
) -> Option<JniCallObjectMethod> {
    debug("Called set_jni_call_object_method");
    JNI_CALL_OBJECT_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_object_method()
}

pub(crate) fn get_jni_call_object_method() -> Option<JniCallObjectMethod> {
    JNI_CALL_OBJECT_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_void_method(j: Option<JniCallVoidMethod>) -> Option<JniCallVoidMethod> {
    debug("Called set_jni_call_void_method");
    JNI_CALL_VOID_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_void_method()
}

pub(crate) fn set_jni_call_byte_method(j: Option<JniCallByteMethod>) -> Option<JniCallByteMethod> {
    debug("Called set_jni_call_byte_method");
    JNI_CALL_BYTE_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_byte_method()
}

pub(crate) fn get_jni_call_byte_method() -> Option<JniCallByteMethod> {
    JNI_CALL_BYTE_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_short_method(
    j: Option<JniCallShortMethod>,
) -> Option<JniCallShortMethod> {
    debug("Called set_jni_call_short_method");
    JNI_CALL_SHORT_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_short_method()
}

pub(crate) fn get_jni_call_short_method() -> Option<JniCallShortMethod> {
    JNI_CALL_SHORT_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_char_method(
    j: Option<JniCallCharMethod>,
) -> Option<JniCallCharMethod> {
    debug("Called set_jni_call_char_method");
    JNI_CALL_CHAR_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_char_method()
}

pub(crate) fn get_jni_call_char_method() -> Option<JniCallCharMethod> {
    JNI_CALL_CHAR_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_int_method(j: Option<JniCallIntMethod>) -> Option<JniCallIntMethod> {
    debug("Called set_jni_call_int_method");
    JNI_CALL_INT_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_int_method()
}

pub(crate) fn get_jni_call_int_method() -> Option<JniCallIntMethod> {
    JNI_CALL_INT_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_long_method(j: Option<JniCallLongMethod>) -> Option<JniCallLongMethod> {
    debug("Called set_jni_call_long_method");
    JNI_CALL_LONG_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_long_method()
}

pub(crate) fn get_jni_call_long_method() -> Option<JniCallLongMethod> {
    JNI_CALL_LONG_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_float_method(
    j: Option<JniCallFloatMethod>,
) -> Option<JniCallFloatMethod> {
    debug("Called set_jni_call_float_method");
    JNI_CALL_FLOAT_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_float_method()
}

pub(crate) fn get_jni_call_float_method() -> Option<JniCallFloatMethod> {
    JNI_CALL_FLOAT_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_double_method(
    j: Option<JniCallDoubleMethod>,
) -> Option<JniCallDoubleMethod> {
    debug("Called set_jni_call_double_method");
    JNI_CALL_DOUBLE_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_double_method()
}

pub(crate) fn get_jni_call_double_method() -> Option<JniCallDoubleMethod> {
    JNI_CALL_DOUBLE_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn get_jni_call_void_method() -> Option<JniCallVoidMethod> {
    JNI_CALL_VOID_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_call_static_object_method(
    j: Option<JniCallStaticObjectMethod>,
) -> Option<JniCallStaticObjectMethod> {
    debug("Called set_jni_call_static_object_method");
    JNI_CALL_STATIC_OBJECT_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_static_object_method()
}

pub(crate) fn get_jni_call_static_object_method() -> Option<JniCallStaticObjectMethod> {
    JNI_CALL_STATIC_OBJECT_METHOD.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_get_array_length(
    j: Option<JniGetArrayLength>,
) -> Option<JniGetArrayLength> {
    debug("Called set_jni_get_array_length");
    JNI_GET_ARRAY_LENGTH.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_get_array_length()
}

pub(crate) fn get_jni_get_array_length() -> Option<JniGetArrayLength> {
    JNI_GET_ARRAY_LENGTH.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_new_object_array(j: Option<JniNewObjectArray>) -> Option<JniNewObjectArray> {
    debug("Called set_jni_new_object_array");

    JNI_NEW_OBJECT_ARRAY.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_object_array()
}

pub(crate) fn get_jni_new_object_array() -> Option<JniNewObjectArray> {
    JNI_NEW_OBJECT_ARRAY.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_set_object_array_element(
    j: Option<JniSetObjectArrayElement>,
) -> Option<JniSetObjectArrayElement> {
    debug("Called set_jni_set_object_array_element");
    JNI_SET_OBJECT_ARRAY_ELEMENT.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_set_object_array_element()
}

pub(crate) fn get_jni_set_object_array_element() -> Option<JniSetObjectArrayElement> {
    JNI_SET_OBJECT_ARRAY_ELEMENT.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_exception_check(j: Option<JniExceptionCheck>) -> Option<JniExceptionCheck> {
    debug("Called set_jni_exception_check");
    JNI_EXCEPTION_CHECK.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_exception_check()
}

pub(crate) fn get_jni_exception_check() -> Option<JniExceptionCheck> {
    JNI_EXCEPTION_CHECK.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_exception_describe(
    j: Option<JniExceptionDescribe>,
) -> Option<JniExceptionDescribe> {
    debug("Called set_jni_exception_describe");
    JNI_EXCEPTION_DESCRIBE.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_exception_describe()
}

pub(crate) fn get_jni_exception_describe() -> Option<JniExceptionDescribe> {
    JNI_EXCEPTION_DESCRIBE.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_exception_clear(j: Option<JniExceptionClear>) -> Option<JniExceptionClear> {
    debug("Called set_jni_exception_clear");
    JNI_EXCEPTION_CLEAR.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_exception_clear()
}

pub(crate) fn get_jni_exception_clear() -> Option<JniExceptionClear> {
    JNI_EXCEPTION_CLEAR.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_delete_local_ref(j: Option<JniDeleteLocalRef>) -> Option<JniDeleteLocalRef> {
    debug("Called set_jni_delete_local_ref");
    JNI_DELETE_LOCAL_REF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_delete_local_ref()
}

pub(crate) fn get_jni_delete_local_ref() -> Option<JniDeleteLocalRef> {
    JNI_DELETE_LOCAL_REF.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_delete_global_ref(
    j: Option<JniDeleteGlobalRef>,
) -> Option<JniDeleteGlobalRef> {
    debug("Called set_jni_delete_global_ref");
    JNI_DELETE_GLOBAL_REF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_delete_global_ref()
}

pub(crate) fn get_jni_delete_global_ref() -> Option<JniDeleteGlobalRef> {
    JNI_DELETE_GLOBAL_REF.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_new_global_ref(j: Option<JniNewGlobalRef>) -> Option<JniNewGlobalRef> {
    debug("Called set_jni_new_global_ref");
    JNI_NEW_GLOBAL_REF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_global_ref()
}

pub(crate) fn get_jni_new_global_ref() -> Option<JniNewGlobalRef> {
    JNI_NEW_GLOBAL_REF.with(|opt| *opt.borrow())
}

pub(crate) fn set_jni_throw_new(j: Option<JniThrowNew>) -> Option<JniThrowNew> {
    debug("Called set_jni_throw_new");
    JNI_THROW_NEW.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_throw_new()
}

pub(crate) fn get_jni_throw_new() -> Option<JniThrowNew> {
    JNI_THROW_NEW.with(|opt| *opt.borrow())
}

pub(crate) fn set_is_same_object(j: Option<JniIsSameObject>) -> Option<JniIsSameObject> {
    debug("Called set_is_same_object");
    JNI_IS_SAME_OBJECT.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_is_same_object()
}

pub(crate) fn get_is_same_object() -> Option<JniIsSameObject> {
    JNI_IS_SAME_OBJECT.with(|opt| *opt.borrow())
}

pub(crate) fn set_factory_class(j: jclass) {
    debug("Called set_factory_class");
    FACTORY_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_class() -> errors::Result<jclass> {
    get_cached!(
        FACTORY_CLASS,
        {
            let env = get_thread_local_env()?;
            let c = tweaks::find_class(env, INST_CLASS_NAME)?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_factory_class
    )
}

pub(crate) fn set_invocation_arg_class(j: jclass) {
    debug("Called set_invocation_arg_class");
    INVOCATION_ARG_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invocation_arg_class() -> errors::Result<jclass> {
    get_cached!(
        INVOCATION_ARG_CLASS,
        {
            let env = get_thread_local_env()?;
            let c = tweaks::find_class(env, "org/astonbitecode/j4rs/api/dtos/InvocationArg")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_invocation_arg_class
    )
}

#[allow(dead_code)]
pub(crate) fn set_factory_constructor_method(j: jmethodID) {
    debug("Called set_factory_constructor_method");
    FACTORY_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_factory_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        FACTORY_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string("()V");
            // The constructor of `NativeInstantiationImpl`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_factory_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_factory_constructor_method
    )
}

pub(crate) fn set_factory_instantiate_method(j: jmethodID) {
    debug("Called set_factory_instantiate_method");
    FACTORY_INSTANTIATE_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_instantiate_method() -> errors::Result<jmethodID> {
    get_cached!(
        FACTORY_INSTANTIATE_METHOD,
        {
            let env = get_thread_local_env()?;
            let instantiate_method_signature = format!(
                "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME
            );
            let cstr1 = utils::to_c_string("instantiate");
            let cstr2 = utils::to_c_string(&instantiate_method_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_static_method_id())?)(
                    env,
                    get_factory_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_factory_instantiate_method
    )
}

pub(crate) fn set_factory_create_for_static_method(j: jmethodID) {
    debug("Called set_factory_create_for_static_method");
    FACTORY_CREATE_FOR_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_create_for_static_method() -> errors::Result<jmethodID> {
    get_cached!(
        FACTORY_CREATE_FOR_STATIC_METHOD,
        {
            let env = get_thread_local_env()?;
            let create_for_static_method_signature =
                format!("(Ljava/lang/String;)L{};", INVO_IFACE_NAME);

            let cstr1 = utils::to_c_string("createForStatic");
            let cstr2 = utils::to_c_string(&create_for_static_method_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_static_method_id())?)(
                    env,
                    get_factory_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_factory_create_for_static_method
    )
}

pub(crate) fn set_factory_create_java_array_method(j: jmethodID) {
    debug("Called set_factory_create_java_array_method");
    FACTORY_CREATE_JAVA_ARRAY_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_create_java_array_method() -> errors::Result<jmethodID> {
    get_cached!(
        FACTORY_CREATE_JAVA_ARRAY_METHOD,
        {
            let env = get_thread_local_env()?;

            let create_java_array_method_signature = format!(
                "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME
            );
            let cstr1 = utils::to_c_string("createJavaArray");
            let cstr2 = utils::to_c_string(&create_java_array_method_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_static_method_id())?)(
                    env,
                    get_factory_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_factory_create_java_array_method
    )
}

pub(crate) fn set_factory_create_java_list_method(j: jmethodID) {
    debug("Called set_factory_create_java_list_method");
    FACTORY_CREATE_JAVA_LIST_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_create_java_list_method() -> errors::Result<jmethodID> {
    get_cached!(
        FACTORY_CREATE_JAVA_LIST_METHOD,
        {
            let env = get_thread_local_env()?;

            let create_java_list_method_signature = format!(
                "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME
            );
            let cstr1 = utils::to_c_string("createJavaList");
            let cstr2 = utils::to_c_string(&create_java_list_method_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_static_method_id())?)(
                    env,
                    get_factory_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_factory_create_java_list_method
    )
}

pub(crate) fn set_factory_create_java_map_method(j: jmethodID) {
    debug("Called set_factory_create_java_map_method");
    FACTORY_CREATE_JAVA_MAP_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_create_java_map_method() -> errors::Result<jmethodID> {
    get_cached!(
        FACTORY_CREATE_JAVA_MAP_METHOD,
        {
            let env = get_thread_local_env()?;

            let create_java_map_method_signature = format!(
                "(Ljava/lang/String;Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME);
            let cstr1 = utils::to_c_string("createJavaMap");
            let cstr2 = utils::to_c_string(&create_java_map_method_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_static_method_id())?)(
                    env,
                    get_factory_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_factory_create_java_map_method
    )
}

pub(crate) fn set_java_instance_base_class(j: jclass) {
    debug("Called set_java_instance_base_class");
    JAVA_INSTANCE_BASE_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_java_instance_base_class() -> errors::Result<jclass> {
    get_cached!(
        JAVA_INSTANCE_BASE_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, INVO_BASE_NAME)?;
            let j = jni_utils::create_global_ref_from_local_ref(c, env)?;

            j
        },
        set_java_instance_base_class
    )
}

pub(crate) fn set_java_instance_class(j: jclass) {
    debug("Called set_java_instance_class");
    JAVA_INSTANCE_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_java_instance_class() -> errors::Result<jclass> {
    get_cached!(
        JAVA_INSTANCE_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, INVO_IFACE_NAME)?;
            let j = jni_utils::create_global_ref_from_local_ref(c, env)?;

            j
        },
        set_java_instance_class
    )
}

pub(crate) fn set_invoke_method(j: jmethodID) {
    debug("Called set_invoke_method");
    INVOKE_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invoke_method() -> errors::Result<jmethodID> {
    get_cached!(
        INVOKE_METHOD,
        {
            let env = get_thread_local_env()?;

            let invoke_method_signature = format!(
                "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME
            );
            // Get the method ID for the `Instance.invoke`
            let cstr1 = utils::to_c_string("invoke");
            let cstr2 = utils::to_c_string(invoke_method_signature.as_ref());
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_invoke_method
    )
}

pub(crate) fn set_invoke_static_method(j: jmethodID) {
    debug("Called set_invoke_static_method");
    INVOKE_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invoke_static_method() -> errors::Result<jmethodID> {
    get_cached!(
        INVOKE_STATIC_METHOD,
        {
            let env = get_thread_local_env()?;

            let invoke_static_method_signature = format!(
                "(Ljava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)L{};",
                INVO_IFACE_NAME
            );
            let cstr1 = utils::to_c_string("invokeStatic");
            let cstr2 = utils::to_c_string(invoke_static_method_signature.as_ref());
            // Get the method ID for the `Instance.invokeStatic`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_invoke_static_method
    )
}

pub(crate) fn set_invoke_to_channel_method(j: jmethodID) {
    debug("Called set_invoke_to_channel_method");
    INVOKE_TO_CHANNEL_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invoke_to_channel_method() -> errors::Result<jmethodID> {
    get_cached!(
        INVOKE_TO_CHANNEL_METHOD,
        {
            let env = get_thread_local_env()?;

            let invoke_to_channel_method_signature =
                "(JLjava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)V";
            let cstr1 = utils::to_c_string("invokeToChannel");
            let cstr2 = utils::to_c_string(&invoke_to_channel_method_signature);
            // Get the method ID for the `Instance.invokeToChannel`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_invoke_to_channel_method
    )
}

pub(crate) fn set_invoke_async_method(j: jmethodID) {
    debug("Called set_invoke_async_method");
    INVOKE_ASYNC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invoke_async_method() -> errors::Result<jmethodID> {
    get_cached!(
        INVOKE_ASYNC_METHOD,
        {
            let env = get_thread_local_env()?;

            let invoke_to_channel_method_signature =
                "(JLjava/lang/String;[Lorg/astonbitecode/j4rs/api/dtos/InvocationArg;)V";
            let cstr1 = utils::to_c_string("invokeAsyncToChannel");
            let cstr2 = utils::to_c_string(&invoke_to_channel_method_signature);
            // Get the method ID for the `Instance.invokeToChannel`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_invoke_async_method
    )
}

pub(crate) fn set_init_callback_channel_method(j: jmethodID) {
    debug("Called set_init_callback_channel_method");
    INIT_CALLBACK_CHANNEL_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_init_callback_channel_method() -> errors::Result<jmethodID> {
    get_cached!(
        INIT_CALLBACK_CHANNEL_METHOD,
        {
            let env = get_thread_local_env()?;

            let init_callback_channel_method_signature = "(J)V";
            let cstr1 = utils::to_c_string("initializeCallbackChannel");
            let cstr2 = utils::to_c_string(&init_callback_channel_method_signature);
            // Get the method ID for the `Instance.initializeCallbackChannel`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_init_callback_channel_method
    )
}

pub(crate) fn set_field_method(j: jmethodID) {
    debug("Called set_field_method");
    FIELD_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_field_method() -> errors::Result<jmethodID> {
    get_cached!(
        FIELD_METHOD,
        {
            let env = get_thread_local_env()?;

            let field_method_signature = format!("(Ljava/lang/String;)L{};", INVO_IFACE_NAME);
            let cstr1 = utils::to_c_string("field");
            let cstr2 = utils::to_c_string(field_method_signature.as_ref());
            // Get the method ID for the `Instance.field`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_field_method
    )
}

pub(crate) fn set_clone_static_method(j: jmethodID) {
    debug("Called set_clone_static_method");
    CLONE_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_clone_static_method() -> errors::Result<jmethodID> {
    get_cached!(
        CLONE_STATIC_METHOD,
        {
            let env = get_thread_local_env()?;

            let clone_method_signature = format!("(L{};)L{};", INVO_IFACE_NAME, INVO_IFACE_NAME);
            let cstr1 = utils::to_c_string("cloneInstance");
            let cstr2 = utils::to_c_string(clone_method_signature.as_ref());
            // Get the method ID for the `Instance.clone`
            let j = unsafe {
                (opt_to_res(get_jni_get_static_method_id())?)(
                    env,
                    get_class_to_invoke_clone_and_cast()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_clone_static_method
    )
}

pub(crate) fn set_cast_static_method(j: jmethodID) {
    debug("Called set_cast_static_method");
    CAST_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_cast_static_method() -> errors::Result<jmethodID> {
    get_cached!(
        CAST_STATIC_METHOD,
        {
            let env = get_thread_local_env()?;

            let cast_method_signature = format!(
                "(L{};Ljava/lang/String;)L{};",
                INVO_IFACE_NAME, INVO_IFACE_NAME
            );
            let cstr1 = utils::to_c_string("cast");
            let cstr2 = utils::to_c_string(cast_method_signature.as_ref());

            // Get the method ID for the `Instance.cast`
            let j = unsafe {
                (opt_to_res(get_jni_get_static_method_id())?)(
                    env,
                    get_class_to_invoke_clone_and_cast()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_cast_static_method
    )
}

pub(crate) fn set_get_json_method(j: jmethodID) {
    debug("Called set_get_json_method");
    GET_JSON_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_get_json_method() -> errors::Result<jmethodID> {
    get_cached!(
        GET_JSON_METHOD,
        {
            let env = get_thread_local_env()?;

            let get_json_method_signature = "()Ljava/lang/String;";
            let cstr1 = utils::to_c_string("getJson");
            let cstr2 = utils::to_c_string(get_json_method_signature.as_ref());

            // Get the method ID for the `Instance.getJson`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_get_json_method
    )
}

pub(crate) fn set_get_object_class_name_method(j: jmethodID) {
    debug("Called set_get_object_class_name_method");
    GET_OBJECT_CLASS_NAME_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_get_object_class_name_method() -> errors::Result<jmethodID> {
    get_cached!(
        GET_OBJECT_CLASS_NAME_METHOD,
        {
            let env = get_thread_local_env()?;

            let get_object_class_name_method_signature = "()Ljava/lang/String;";
            let cstr1 = utils::to_c_string("getObjectClassName");
            let cstr2 = utils::to_c_string(get_object_class_name_method_signature.as_ref());

            // Get the method ID for the `Instance.getObjectClass`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_get_object_class_name_method
    )
}

pub(crate) fn set_get_object_method(j: jmethodID) {
    debug("Called set_get_object_method");
    GET_OBJECT_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_get_object_method() -> errors::Result<jmethodID> {
    get_cached!(
        GET_OBJECT_METHOD,
        {
            let env = get_thread_local_env()?;

            let get_object_method_signature = "()Ljava/lang/Object;";
            let cstr1 = utils::to_c_string("getObject");
            let cstr2 = utils::to_c_string(get_object_method_signature.as_ref());

            // Get the method ID for the `Instance.getObject`
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_java_instance_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_get_object_method
    )
}

pub(crate) fn set_inv_arg_java_constructor_method(j: jmethodID) {
    debug("Called set_inv_arg_java_constructor_method");
    INV_ARG_JAVA_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_inv_arg_java_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        INV_ARG_JAVA_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let inv_arg_java_constructor_method_signature =
                format!("(Ljava/lang/String;L{};)V", INVO_IFACE_NAME);
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&inv_arg_java_constructor_method_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_invocation_arg_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_inv_arg_java_constructor_method
    )
}

pub(crate) fn set_inv_arg_rust_constructor_method(j: jmethodID) {
    debug("Called set_inv_arg_rust_constructor_method");
    INV_ARG_RUST_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_inv_arg_rust_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        INV_ARG_RUST_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string("(Ljava/lang/String;Ljava/lang/String;)V");
            let j = unsafe {
                let m = opt_to_res(get_jni_get_method_id())?;
                let invargclass = get_invocation_arg_class()?;
                (m)(env, invargclass, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_inv_arg_rust_constructor_method
    )
}

pub(crate) fn set_inv_arg_basic_rust_constructor_method(j: jmethodID) {
    debug("Called set_inv_arg_basic_rust_constructor_method");
    INV_ARG_BASIC_RUST_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_inv_arg_basic_rust_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        INV_ARG_BASIC_RUST_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let inv_arg_basic_rust_constructor_method_signature =
                "(Ljava/lang/String;Ljava/lang/Object;)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&inv_arg_basic_rust_constructor_method_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(
                    env,
                    get_invocation_arg_class()?,
                    cstr1,
                    cstr2,
                )
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_inv_arg_basic_rust_constructor_method
    )
}

pub(crate) fn set_class_to_invoke_clone_and_cast(j: jclass) {
    debug("Called set_class_to_invoke_clone_and_cast");
    CLASS_TO_INVOKE_CLONE_AND_CAST.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_class_to_invoke_clone_and_cast() -> errors::Result<jclass> {
    get_cached!(
        CLASS_TO_INVOKE_CLONE_AND_CAST,
        {
            // The class to invoke the cloneInstance into, is not the same in Android target os.
            // The java_instance_base_class is used because of Java7 compatibility issues in Android.
            // In Java8 and later, the static implementation in the interfaces is used. This is not supported in Java7
            // and there is a base class created for this reason.
            let j = if cfg!(target_os = "android") {
                get_java_instance_base_class()?
            } else {
                get_java_instance_class()?
            };

            j
        },
        set_class_to_invoke_clone_and_cast
    )
}

pub(crate) fn set_integer_class(j: jclass) {
    debug("Called set_integer_class");
    INTEGER_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_integer_class() -> errors::Result<jclass> {
    get_cached!(
        INTEGER_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/Integer")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_integer_class
    )
}

pub(crate) fn set_integer_constructor_method(j: jmethodID) {
    debug("Called set_integer_constructor_method");
    INTEGER_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_integer_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        INTEGER_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let constructor_signature = "(I)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&constructor_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_integer_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_integer_constructor_method
    )
}

pub(crate) fn set_integer_to_int_method(j: jmethodID) {
    debug("Called set_integer_to_int_method");
    INTEGER_TO_INT_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_integer_to_int_method() -> errors::Result<jmethodID> {
    get_cached!(
        INTEGER_TO_INT_METHOD,
        {
            let env = get_thread_local_env()?;

            let to_int_signature = "()I";
            let cstr1 = utils::to_c_string("intValue");
            let cstr2 = utils::to_c_string(&to_int_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_integer_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_integer_to_int_method
    )
}

pub(crate) fn set_long_class(j: jclass) {
    debug("Called set_long_class");
    LONG_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_long_class() -> errors::Result<jclass> {
    get_cached!(
        LONG_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/Long")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_long_class
    )
}

pub(crate) fn set_invocation_exception_class(j: jclass) {
    debug("Called set_invocation_exception_class");
    LONG_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invocation_exception_class() -> errors::Result<jclass> {
    get_cached!(
        INVOCATION_EXCEPTION_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "org/astonbitecode/j4rs/errors/InvocationException")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_invocation_exception_class
    )
}

pub(crate) fn set_long_constructor_method(j: jmethodID) {
    debug("Called set_long_constructor_method");
    LONG_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_long_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        LONG_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let constructor_signature = "(J)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&constructor_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_long_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_long_constructor_method
    )
}

pub(crate) fn set_long_to_long_method(j: jmethodID) {
    debug("Called set_long_to_long_method");
    LONG_TO_LONG_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_long_to_long_method() -> errors::Result<jmethodID> {
    get_cached!(
        LONG_TO_LONG_METHOD,
        {
            let env = get_thread_local_env()?;

            let signature = "()J";
            let cstr1 = utils::to_c_string("longValue");
            let cstr2 = utils::to_c_string(&signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_long_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_long_to_long_method
    )
}

pub(crate) fn set_short_class(j: jclass) {
    debug("Called set_short_class");
    SHORT_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_short_class() -> errors::Result<jclass> {
    get_cached!(
        SHORT_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/Short")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_short_class
    )
}

pub(crate) fn set_short_constructor_method(j: jmethodID) {
    debug("Called set_short_constructor_method");
    SHORT_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_short_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        SHORT_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let constructor_signature = "(S)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&constructor_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_short_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_short_constructor_method
    )
}

pub(crate) fn set_short_to_short_method(j: jmethodID) {
    debug("Called set_short_to_short_method");
    SHORT_TO_SHORT_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_short_to_short_method() -> errors::Result<jmethodID> {
    get_cached!(
        SHORT_TO_SHORT_METHOD,
        {
            let env = get_thread_local_env()?;

            let signature = "()S";
            let cstr1 = utils::to_c_string("shortValue");
            let cstr2 = utils::to_c_string(&signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_short_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_short_to_short_method
    )
}

pub(crate) fn set_character_class(j: jclass) {
    debug("Called set_character_class");
    CHARACTER_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_character_class() -> errors::Result<jclass> {
    get_cached!(
        CHARACTER_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/Character")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_character_class
    )
}

pub(crate) fn set_character_constructor_method(j: jmethodID) {
    debug("Called set_character_constructor_method");
    CHARACTER_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_character_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        CHARACTER_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let constructor_signature = "(C)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&constructor_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_character_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_character_constructor_method
    )
}

pub(crate) fn set_character_to_char_method(j: jmethodID) {
    debug("Called set_character_to_char_method");
    CHARACTER_TO_CHAR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_character_to_char_method() -> errors::Result<jmethodID> {
    get_cached!(
        CHARACTER_TO_CHAR_METHOD,
        {
            let env = get_thread_local_env()?;

            let signature = "()C";
            let cstr1 = utils::to_c_string("charValue");
            let cstr2 = utils::to_c_string(&signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_character_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_character_to_char_method
    )
}

pub(crate) fn set_byte_class(j: jclass) {
    debug("Called set_byte_class");
    BYTE_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_byte_class() -> errors::Result<jclass> {
    get_cached!(
        BYTE_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/Byte")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_byte_class
    )
}

pub(crate) fn set_byte_constructor_method(j: jmethodID) {
    debug("Called set_byte_constructor_method");
    BYTE_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_byte_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        BYTE_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let constructor_signature = "(B)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&constructor_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_byte_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_byte_constructor_method
    )
}

pub(crate) fn set_byte_to_byte_method(j: jmethodID) {
    debug("Called set_byte_to_byte_method");
    BYTE_TO_BYTE_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_byte_to_byte_method() -> errors::Result<jmethodID> {
    get_cached!(
        BYTE_TO_BYTE_METHOD,
        {
            let env = get_thread_local_env()?;

            let signature = "()B";
            let cstr1 = utils::to_c_string("byteValue");
            let cstr2 = utils::to_c_string(&signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_byte_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_byte_to_byte_method
    )
}

#[allow(dead_code)]
pub(crate) fn set_float_class(j: jclass) {
    debug("Called set_float_class");
    FLOAT_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_float_class() -> errors::Result<jclass> {
    get_cached!(
        FLOAT_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/Float")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_float_class
    )
}

#[allow(dead_code)]
pub(crate) fn set_float_constructor_method(j: jmethodID) {
    debug("Called set_float_constructor_method");
    FLOAT_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_float_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        FLOAT_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let constructor_signature = "(F)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&constructor_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_float_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_float_constructor_method
    )
}

#[allow(dead_code)]
pub(crate) fn set_float_to_float_method(j: jmethodID) {
    debug("Called set_float_to_float_method");
    FLOAT_TO_FLOAT_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_float_to_float_method() -> errors::Result<jmethodID> {
    get_cached!(
        FLOAT_TO_FLOAT_METHOD,
        {
            let env = get_thread_local_env()?;

            let signature = "()F";
            let cstr1 = utils::to_c_string("floatValue");
            let cstr2 = utils::to_c_string(&signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_float_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_float_to_float_method
    )
}

#[allow(dead_code)]
pub(crate) fn set_double_class(j: jclass) {
    debug("Called set_double_class");
    DOUBLE_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_double_class() -> errors::Result<jclass> {
    get_cached!(
        DOUBLE_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/Double")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_double_class
    )
}

#[allow(dead_code)]
pub(crate) fn set_double_constructor_method(j: jmethodID) {
    debug("Called set_double_constructor_method");
    DOUBLE_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_double_constructor_method() -> errors::Result<jmethodID> {
    get_cached!(
        DOUBLE_CONSTRUCTOR_METHOD,
        {
            let env = get_thread_local_env()?;

            let constructor_signature = "(D)V";
            let cstr1 = utils::to_c_string("<init>");
            let cstr2 = utils::to_c_string(&constructor_signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_double_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_double_constructor_method
    )
}

#[allow(dead_code)]
pub(crate) fn set_double_to_double_method(j: jmethodID) {
    debug("Called set_double_to_double_method");
    DOUBLE_TO_DOUBLE_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_double_to_double_method() -> errors::Result<jmethodID> {
    get_cached!(
        DOUBLE_TO_DOUBLE_METHOD,
        {
            let env = get_thread_local_env()?;

            let signature = "()D";
            let cstr1 = utils::to_c_string("doubleValue");
            let cstr2 = utils::to_c_string(&signature);
            let j = unsafe {
                (opt_to_res(get_jni_get_method_id())?)(env, get_double_class()?, cstr1, cstr2)
            };
            utils::drop_c_string(cstr1);
            utils::drop_c_string(cstr2);

            j
        },
        set_double_to_double_method
    )
}

#[allow(dead_code)]
pub(crate) fn set_string_class(j: jclass) {
    debug("Called set_string_class");
    STRING_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

#[allow(dead_code)]
pub(crate) fn get_string_class() -> errors::Result<jclass> {
    get_cached!(
        STRING_CLASS,
        {
            let env = get_thread_local_env()?;

            let c = tweaks::find_class(env, "java/lang/String")?;
            jni_utils::create_global_ref_from_local_ref(c, env)?
        },
        set_string_class
    )
}
