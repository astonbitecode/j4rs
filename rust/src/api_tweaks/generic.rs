use jni_sys::{
    JavaVM,
    jint,
    JNI_GetCreatedJavaVMs,
    jsize,
};

pub fn get_created_java_vms(vm_buf: *mut *mut JavaVM, buf_len: jsize, n_vms: *mut jsize) -> jint {
    unsafe { JNI_GetCreatedJavaVMs(vm_buf, buf_len, n_vms) }
}