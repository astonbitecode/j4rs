use std::os::raw::c_void;

use jni_sys::{
    JavaVM,
    jclass,
    jint,
    JNI_CreateJavaVM,
    JNI_GetCreatedJavaVMs,
    JNIEnv,
    jsize,
};

use crate::utils;

pub(crate) fn get_created_java_vms(vm_buf: &mut Vec<*mut JavaVM>, buf_len: jsize, n_vms: *mut jsize) -> jint {
    unsafe { JNI_GetCreatedJavaVMs(vm_buf.as_mut_ptr(), buf_len, n_vms) }
}

pub(crate) fn create_java_vm(
    jvm: *mut *mut JavaVM,
    penv: *mut *mut c_void,
    args: *mut c_void,
) -> jint {
    unsafe { JNI_CreateJavaVM(jvm, penv, args) }
}

pub(crate) fn find_class(env: *mut JNIEnv, classname: &str) -> jclass {
    unsafe {
        let fc = ((**env).FindClass).expect("Could not dereference the JNIEnv to get the FindClass");
        (fc)(
            env,
            utils::to_java_string(classname),
        )
    }
}