use jni_sys::{
    JavaVM,
    jint,
    jsize,
};

pub fn get_created_java_vms(_vm_buf: *mut *mut JavaVM, _buf_len: jsize, _n_vms: *mut jsize) -> jint {
//    panic!("get_created_java_vms is not supported for Android. Please create a Jvm using some other function. e.g.: `j4rs::Jvm::try_from(env)`, where env is a `*mut JNIEnv`");
    1
}