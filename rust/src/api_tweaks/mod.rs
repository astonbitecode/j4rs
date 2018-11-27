use jni_sys::{JavaVM, jint, jsize, JNIEnv, jclass};
use std::os::raw::c_void;


#[cfg(not(target_os = "android"))]
mod generic;

#[cfg(not(target_os = "android"))]
pub fn get_created_java_vms(vm_buf: &mut Vec<*mut JavaVM>, buf_len: jsize, n_vms: *mut jsize) -> jint {
    generic::get_created_java_vms(vm_buf, buf_len, n_vms)
}

#[cfg(not(target_os = "android"))]
pub fn set_java_vm(_: *mut JavaVM) {}

#[cfg(not(target_os = "android"))]
pub fn create_java_vm(
    pvm: *mut *mut JavaVM,
    penv: *mut *mut c_void,
    args: *mut c_void,
) -> jint { generic::create_java_vm(pvm, penv, args) }

#[cfg(not(target_os = "android"))]
pub fn find_class(env: *mut JNIEnv, classname: &str) -> jclass {
    generic::find_class(env, classname)
}
// ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++ //

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
pub fn get_created_java_vms(vm_buf: &mut Vec<*mut JavaVM>, buf_len: jsize, n_vms: *mut jsize) -> jint {
    android::get_created_java_vms(vm_buf, buf_len, n_vms)
}

#[cfg(target_os = "android")]
pub fn set_java_vm(java_vm: *mut JavaVM) {
    android::set_java_vm(java_vm);
}

#[cfg(target_os = "android")]
pub fn create_java_vm(
    pvm: *mut *mut JavaVM,
    penv: *mut *mut c_void,
    args: *mut c_void,
) -> jint { android::create_java_vm(pvm, penv, args) }

#[cfg(target_os = "android")]
pub fn find_class(env: *mut JNIEnv, classname: &str) -> jclass {
    android::find_class(env, classname)
}