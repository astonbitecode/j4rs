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

use jni_sys::{
    self,
    jboolean,
    jclass,
    jmethodID,
    JNIEnv,
    jobject,
    jobjectArray,
    jsize,
    jstring,
};
use libc::c_char;

use crate::api::Jvm;
use crate::errors;

pub(crate) const INST_CLASS_NAME: &'static str = "org/astonbitecode/j4rs/api/instantiation/NativeInstantiationImpl";
pub(crate) const INVO_BASE_NAME: &'static str = "org/astonbitecode/j4rs/api/NativeInvocationBase";
pub(crate) const INVO_IFACE_NAME: &'static str = "org/astonbitecode/j4rs/api/NativeInvocation";
pub(crate) const UNKNOWN_FOR_RUST: &'static str = "known_in_java_world";
pub(crate) const J4RS_ARRAY: &'static str = "org.astonbitecode.j4rs.api.dtos.Array";

pub(crate) type JniGetMethodId = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, *const c_char, *const c_char) -> *mut jni_sys::_jmethodID;
pub(crate) type JniGetStaticMethodId = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, *const c_char, *const c_char) -> *mut jni_sys::_jmethodID;
#[allow(non_snake_case)]
pub(crate) type JniNewObject = unsafe extern "C" fn(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, ...) -> jobject;
pub(crate) type JniNewStringUTF = unsafe extern "system" fn(env: *mut JNIEnv, utf: *const c_char) -> jstring;
#[allow(non_snake_case)]
pub(crate) type JniGetStringUTFChars = unsafe extern "system" fn(env: *mut JNIEnv, str: jstring, isCopy: *mut jboolean) -> *const c_char;
#[allow(non_snake_case)]
pub(crate) type JniCallObjectMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
#[allow(non_snake_case)]
pub(crate) type JniCallVoidMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...);
pub(crate) type JniCallStaticObjectMethod = unsafe extern "C" fn(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, ...) -> jobject;
pub(crate) type JniNewObjectArray = unsafe extern "system" fn(env: *mut JNIEnv, len: jsize, clazz: jclass, init: jobject) -> jobjectArray;
pub(crate) type JniSetObjectArrayElement = unsafe extern "system" fn(*mut *const jni_sys::JNINativeInterface_, *mut jni_sys::_jobject, i32, *mut jni_sys::_jobject);
pub(crate) type JniExceptionCheck = unsafe extern "system" fn(_: *mut JNIEnv) -> jboolean;
pub(crate) type JniExceptionDescribe = unsafe extern "system" fn(_: *mut JNIEnv);
pub(crate) type JniExceptionClear = unsafe extern "system" fn(_: *mut JNIEnv);
pub(crate) type JniDeleteLocalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> ();
pub(crate) type JniDeleteGlobalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> ();
pub(crate) type JniNewGlobalRef = unsafe extern "system" fn(_: *mut JNIEnv, _: jobject) -> jobject;

lazy_static! {
    // Synchronize the creation of Jvm
    pub(crate) static ref MUTEX: Mutex<bool> = Mutex::new(false);
    // If a Jvm is created with defining a jassets_path other than the default, this is set here
    pub(crate) static ref JASSETS_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

thread_local! {
    pub(crate) static JNI_ENV: RefCell<Option<*mut JNIEnv>> = RefCell::new(None);
    pub(crate) static JVM: RefCell<Option<Jvm>> = RefCell::new(None);
    pub(crate) static ACTIVE_JVMS: RefCell<i32> = RefCell::new(0);
    pub(crate) static JNI_GET_METHOD_ID: RefCell<Option<JniGetMethodId>> = RefCell::new(None);
    pub(crate) static JNI_GET_STATIC_METHOD_ID: RefCell<Option<JniGetStaticMethodId>> = RefCell::new(None);
    pub(crate) static JNI_NEW_OBJECT: RefCell<Option<JniNewObject>> = RefCell::new(None);
    pub(crate) static JNI_NEW_STRING_UTF: RefCell<Option<JniNewStringUTF>> = RefCell::new(None);
    pub(crate) static JNI_GET_STRING_UTF_CHARS: RefCell<Option<JniGetStringUTFChars>> = RefCell::new(None);
    pub(crate) static JNI_CALL_OBJECT_METHOD: RefCell<Option<JniCallObjectMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_VOID_METHOD: RefCell<Option<JniCallVoidMethod>> = RefCell::new(None);
    pub(crate) static JNI_CALL_STATIC_OBJECT_METHOD: RefCell<Option<JniCallStaticObjectMethod>> = RefCell::new(None);
    pub(crate) static JNI_NEW_OBJECT_ARRAY: RefCell<Option<JniNewObjectArray>> = RefCell::new(None);
    pub(crate) static JNI_SET_OBJECT_ARRAY_ELEMENT: RefCell<Option<JniSetObjectArrayElement>> = RefCell::new(None);
    pub(crate) static JNI_EXCEPTION_CHECK: RefCell<Option<JniExceptionCheck>> = RefCell::new(None);
    pub(crate) static JNI_EXCEPTION_DESCRIBE: RefCell<Option<JniExceptionDescribe>> = RefCell::new(None);
    pub(crate) static JNI_EXCEPTION_CLEAR: RefCell<Option<JniExceptionClear>> = RefCell::new(None);
    pub(crate) static JNI_DELETE_LOCAL_REF: RefCell<Option<JniDeleteLocalRef>> = RefCell::new(None);
    pub(crate) static JNI_DELETE_GLOBAL_REF: RefCell<Option<JniDeleteGlobalRef>> = RefCell::new(None);
    pub(crate) static JNI_NEW_GLOBAL_REF: RefCell<Option<JniNewGlobalRef>> = RefCell::new(None);
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
    // The `NativeInvocationBase` class.
    // This is optional because it exists only in Android for Java7 compatibility
    // because Java7 does not support static method implementations in interfaces.
    pub(crate) static NATIVE_INVOCATION_BASE_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    // The `NativeInvocation` class.
    pub(crate) static NATIVE_INVOCATION_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    // The Java class for the `InvocationArg`.
    pub(crate) static INVOCATION_ARG_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    // The invoke method
    pub(crate) static INVOKE_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invoke static method
    pub(crate) static INVOKE_STATIC_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invoke to channel method
    pub(crate) static INVOKE_TO_CHANNEL_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
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
    // The invstatic ocation argument constructor method for objects created by Java
    pub(crate) static INV_ARG_JAVA_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invstatic ocation argument constructor method for objects created by Rust
    pub(crate) static INV_ARG_RUST_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // The invstatic ocation argument constructor method for objects of Basic type created by Rust
    pub(crate) static INV_ARG_BASIC_RUST_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    // Basic types definitions
    pub(crate) static INTEGER_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static INTEGER_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static LONG_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static LONG_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static SHORT_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static SHORT_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static BYTE_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static BYTE_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static FLOAT_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static FLOAT_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
    pub(crate) static DOUBLE_CONSTRUCTOR_METHOD: RefCell<Option<jmethodID>> = RefCell::new(None);
    pub(crate) static DOUBLE_CLASS: RefCell<Option<jclass>> = RefCell::new(None);
}

pub(crate) fn add_active_jvm() {
    ACTIVE_JVMS.with(|active_jvms| {
        let active_number = {
            *active_jvms.borrow() + 1
        };
        *active_jvms.borrow_mut() = active_number;
    });
}

pub(crate) fn remove_active_jvm() -> i32 {
    ACTIVE_JVMS.with(|active_jvms| {
        let active_number = {
            *active_jvms.borrow() - 1
        };
        *active_jvms.borrow_mut() = active_number;
        active_number
    })
}

pub(crate) fn get_thread_local_env_opt() -> Option<*mut JNIEnv> {
    JNI_ENV.with(|existing_jni_env_opt| {
        match *existing_jni_env_opt.borrow() {
            Some(env) => Some(env),
            None => None,
        }
    })
}

pub(crate) fn set_thread_local_env(jni_env_opt: Option<*mut JNIEnv>) {
    JNI_ENV.with(|existing_jni_env_opt| {
        *existing_jni_env_opt.borrow_mut() = jni_env_opt;
    });
}

pub(crate) fn get_thread_local_env() -> errors::Result<*mut JNIEnv> {
    match get_thread_local_env_opt() {
        Some(env) => Ok(env),
        None => Err(errors::J4RsError::JavaError(format!("Could not find the JNIEnv in the thread local"))),
    }
}

pub(crate) fn set_jni_get_method_id(j: Option<JniGetMethodId>) -> Option<JniGetMethodId> {
    JNI_GET_METHOD_ID.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_get_method_id()
}

pub(crate) fn get_jni_get_method_id() -> Option<JniGetMethodId> {
    JNI_GET_METHOD_ID.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_get_static_method_id(j: Option<JniGetMethodId>) -> Option<JniGetStaticMethodId> {
    JNI_GET_STATIC_METHOD_ID.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_get_static_method_id()
}

pub(crate) fn get_jni_get_static_method_id() -> Option<JniGetStaticMethodId> {
    JNI_GET_STATIC_METHOD_ID.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_new_object(j: Option<JniNewObject>) -> Option<JniNewObject> {
    JNI_NEW_OBJECT.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_object()
}

pub(crate) fn get_jni_new_object() -> Option<JniNewObject> {
    JNI_NEW_OBJECT.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_new_string_utf(j: Option<JniNewStringUTF>) -> Option<JniNewStringUTF> {
    JNI_NEW_STRING_UTF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_string_utf()
}

pub(crate) fn get_jni_new_string_utf() -> Option<JniNewStringUTF> {
    JNI_NEW_STRING_UTF.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_get_string_utf_chars(j: Option<JniGetStringUTFChars>) -> Option<JniGetStringUTFChars> {
    JNI_GET_STRING_UTF_CHARS.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_get_string_utf_chars()
}

pub(crate) fn get_jni_get_string_utf_chars() -> Option<JniGetStringUTFChars> {
    JNI_GET_STRING_UTF_CHARS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_call_object_method(j: Option<JniCallObjectMethod>) -> Option<JniCallObjectMethod> {
    JNI_CALL_OBJECT_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_object_method()
}

pub(crate) fn get_jni_call_object_method() -> Option<JniCallObjectMethod> {
    JNI_CALL_OBJECT_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_call_void_method(j: Option<JniCallVoidMethod>) -> Option<JniCallVoidMethod> {
    JNI_CALL_VOID_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_void_method()
}

pub(crate) fn get_jni_call_void_method() -> Option<JniCallVoidMethod> {
    JNI_CALL_VOID_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_call_static_object_method(j: Option<JniCallStaticObjectMethod>) -> Option<JniCallStaticObjectMethod> {
    JNI_CALL_STATIC_OBJECT_METHOD.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_call_static_object_method()
}

pub(crate) fn get_jni_call_static_object_method() -> Option<JniCallStaticObjectMethod> {
    JNI_CALL_STATIC_OBJECT_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_new_object_array(j: Option<JniNewObjectArray>) -> Option<JniNewObjectArray> {
    JNI_NEW_OBJECT_ARRAY.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_object_array()
}

pub(crate) fn get_jni_new_object_array() -> Option<JniNewObjectArray> {
    JNI_NEW_OBJECT_ARRAY.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_set_object_array_element(j: Option<JniSetObjectArrayElement>) -> Option<JniSetObjectArrayElement> {
    JNI_SET_OBJECT_ARRAY_ELEMENT.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_set_object_array_element()
}

pub(crate) fn get_jni_set_object_array_element() -> Option<JniSetObjectArrayElement> {
    JNI_SET_OBJECT_ARRAY_ELEMENT.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_exception_check(j: Option<JniExceptionCheck>) -> Option<JniExceptionCheck> {
    JNI_EXCEPTION_CHECK.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_exception_check()
}

pub(crate) fn get_jni_exception_check() -> Option<JniExceptionCheck> {
    JNI_EXCEPTION_CHECK.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_exception_describe(j: Option<JniExceptionDescribe>) -> Option<JniExceptionDescribe> {
    JNI_EXCEPTION_DESCRIBE.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_exception_describe()
}

pub(crate) fn get_jni_exception_describe() -> Option<JniExceptionDescribe> {
    JNI_EXCEPTION_DESCRIBE.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_exception_clear(j: Option<JniExceptionClear>) -> Option<JniExceptionClear> {
    JNI_EXCEPTION_CLEAR.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_exception_clear()
}

pub(crate) fn get_jni_exception_clear() -> Option<JniExceptionClear> {
    JNI_EXCEPTION_CLEAR.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_delete_local_ref(j: Option<JniDeleteLocalRef>) -> Option<JniDeleteLocalRef> {
    JNI_DELETE_LOCAL_REF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_delete_local_ref()
}

pub(crate) fn get_jni_delete_local_ref() -> Option<JniDeleteLocalRef> {
    JNI_DELETE_LOCAL_REF.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_delete_global_ref(j: Option<JniDeleteGlobalRef>) -> Option<JniDeleteGlobalRef> {
    JNI_DELETE_GLOBAL_REF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_delete_global_ref()
}

pub(crate) fn get_jni_delete_global_ref() -> Option<JniDeleteGlobalRef> {
    JNI_DELETE_GLOBAL_REF.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_jni_new_global_ref(j: Option<JniNewGlobalRef>) -> Option<JniNewGlobalRef> {
    JNI_NEW_GLOBAL_REF.with(|opt| {
        *opt.borrow_mut() = j;
    });
    get_jni_new_global_ref()
}

pub(crate) fn get_jni_new_global_ref() -> Option<JniNewGlobalRef> {
    JNI_NEW_GLOBAL_REF.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_factory_class(j: jclass) -> jclass {
    FACTORY_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_factory_class().unwrap()
}

pub(crate) fn get_factory_class() -> Option<jclass> {
    FACTORY_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_invocation_arg_class(j: jclass) -> jclass {
    INVOCATION_ARG_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_invocation_arg_class().unwrap()
}

pub(crate) fn get_invocation_arg_class() -> Option<jclass> {
    INVOCATION_ARG_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_factory_constructor_method(j: jmethodID) -> jmethodID {
    FACTORY_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_factory_constructor_method().unwrap()
}

pub(crate) fn get_factory_constructor_method() -> Option<jmethodID> {
    FACTORY_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_factory_instantiate_method(j: jmethodID) -> jmethodID {
    FACTORY_INSTANTIATE_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_factory_instantiate_method().unwrap()
}

pub(crate) fn get_factory_instantiate_method() -> Option<jmethodID> {
    FACTORY_INSTANTIATE_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_factory_create_for_static_method(j: jmethodID) {
    FACTORY_CREATE_FOR_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_create_for_static_method() -> Option<jmethodID> {
    FACTORY_CREATE_FOR_STATIC_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_factory_create_java_array_method(j: jmethodID) {
    FACTORY_CREATE_JAVA_ARRAY_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_create_java_array_method() -> Option<jmethodID> {
    FACTORY_CREATE_JAVA_ARRAY_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_factory_create_java_list_method(j: jmethodID) {
    FACTORY_CREATE_JAVA_LIST_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_factory_create_java_list_method() -> Option<jmethodID> {
    FACTORY_CREATE_JAVA_LIST_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_native_invocation_base_class(j: jclass) -> jclass {
    NATIVE_INVOCATION_BASE_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_native_invocation_base_class().unwrap()
}

pub(crate) fn get_native_invocation_base_class() -> Option<jclass> {
    NATIVE_INVOCATION_BASE_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_native_invocation_class(j: jclass) -> jclass {
    NATIVE_INVOCATION_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_native_invocation_class().unwrap()
}

pub(crate) fn get_native_invocation_class() -> Option<jclass> {
    NATIVE_INVOCATION_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_invoke_method(j: jmethodID) {
    INVOKE_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invoke_method() -> Option<jmethodID> {
    INVOKE_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_invoke_static_method(j: jmethodID) {
    INVOKE_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invoke_static_method() -> Option<jmethodID> {
    INVOKE_STATIC_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_invoke_to_channel_method(j: jmethodID) {
    INVOKE_TO_CHANNEL_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_invoke_to_channel_method() -> Option<jmethodID> {
    INVOKE_TO_CHANNEL_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_init_callback_channel_method(j: jmethodID) {
    INIT_CALLBACK_CHANNEL_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_init_callback_channel_method() -> Option<jmethodID> {
    INIT_CALLBACK_CHANNEL_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_field_method(j: jmethodID) {
    FIELD_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_field_method() -> Option<jmethodID> {
    FIELD_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_clone_static_method(j: jmethodID) {
    CLONE_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_clone_static_method() -> Option<jmethodID> {
    CLONE_STATIC_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_cast_static_method(j: jmethodID) {
    CAST_STATIC_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_cast_static_method() -> Option<jmethodID> {
    CAST_STATIC_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_get_json_method(j: jmethodID) {
    GET_JSON_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_get_json_method() -> Option<jmethodID> {
    GET_JSON_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_inv_arg_java_constructor_method(j: jmethodID) {
    INV_ARG_JAVA_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_inv_arg_java_constructor_method() -> Option<jmethodID> {
    INV_ARG_JAVA_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_inv_arg_rust_constructor_method(j: jmethodID) {
    INV_ARG_RUST_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_inv_arg_rust_constructor_method() -> Option<jmethodID> {
    INV_ARG_RUST_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_inv_arg_basic_rust_constructor_method(j: jmethodID) {
    INV_ARG_BASIC_RUST_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_inv_arg_basic_rust_constructor_method() -> Option<jmethodID> {
    INV_ARG_BASIC_RUST_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_class_to_invoke_clone_and_cast(j: jclass) -> jclass {
    CLASS_TO_INVOKE_CLONE_AND_CAST.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_class_to_invoke_clone_and_cast().unwrap()
}

pub(crate) fn get_class_to_invoke_clone_and_cast() -> Option<jclass> {
    CLASS_TO_INVOKE_CLONE_AND_CAST.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_integer_class(j: jclass) -> jclass {
    INTEGER_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_integer_class().unwrap()
}

pub(crate) fn get_integer_class() -> Option<jclass> {
    INTEGER_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_integer_constructor_method(j: jmethodID) {
    INTEGER_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_integer_constructor_method() -> Option<jmethodID> {
    INTEGER_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_long_class(j: jclass) -> jclass {
    LONG_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_long_class().unwrap()
}

pub(crate) fn get_long_class() -> Option<jclass> {
    LONG_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_long_constructor_method(j: jmethodID) {
    LONG_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_long_constructor_method() -> Option<jmethodID> {
    LONG_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_short_class(j: jclass) -> jclass {
    SHORT_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_short_class().unwrap()
}

pub(crate) fn get_short_class() -> Option<jclass> {
    SHORT_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_short_constructor_method(j: jmethodID) {
    SHORT_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_short_constructor_method() -> Option<jmethodID> {
    SHORT_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_byte_class(j: jclass) -> jclass {
    BYTE_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_byte_class().unwrap()
}

pub(crate) fn get_byte_class() -> Option<jclass> {
    BYTE_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_byte_constructor_method(j: jmethodID) {
    BYTE_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_byte_constructor_method() -> Option<jmethodID> {
    BYTE_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_float_class(j: jclass) -> jclass {
    FLOAT_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_float_class().unwrap()
}

pub(crate) fn get_float_class() -> Option<jclass> {
    FLOAT_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_float_constructor_method(j: jmethodID) {
    FLOAT_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_float_constructor_method() -> Option<jmethodID> {
    FLOAT_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_double_class(j: jclass) -> jclass {
    DOUBLE_CLASS.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
    get_double_class().unwrap()
}

pub(crate) fn get_double_class() -> Option<jclass> {
    DOUBLE_CLASS.with(|opt| {
        *opt.borrow()
    })
}

pub(crate) fn set_double_constructor_method(j: jmethodID) {
    DOUBLE_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow_mut() = Some(j);
    });
}

pub(crate) fn get_double_constructor_method() -> Option<jmethodID> {
    DOUBLE_CONSTRUCTOR_METHOD.with(|opt| {
        *opt.borrow()
    })
}