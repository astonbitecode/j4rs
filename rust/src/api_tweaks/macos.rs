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

#[link(name = "jvm")]
extern {}

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