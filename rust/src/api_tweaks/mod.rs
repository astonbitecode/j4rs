use jni_sys::{JavaVM, jint, jsize};

#[cfg(not(target_os = "android"))]
mod generic;

#[cfg(not(target_os = "android"))]
pub fn get_created_java_vms(vm_buf: *mut *mut JavaVM, buf_len: jsize, n_vms: *mut jsize) -> jint {
    generic::get_created_java_vms(vm_buf, buf_len, n_vms)
}

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
pub fn get_created_java_vms(vm_buf: *mut *mut JavaVM, buf_len: jsize, n_vms: *mut jsize) -> jint {
    android::get_created_java_vms(vm_buf, buf_len, n_vms)
}