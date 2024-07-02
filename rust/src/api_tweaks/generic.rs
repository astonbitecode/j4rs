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
use std::path::MAIN_SEPARATOR;

use java_locator::{get_jvm_dyn_lib_file_name, locate_jvm_dyn_library};
use jni_sys::{jclass, jint, jsize, JNIEnv, JavaVM};

use crate::{errors, utils};

type JNIGetCreatedJavaVMs =
    unsafe extern "system" fn(vmBuf: *mut *mut JavaVM, bufLen: jsize, nVMs: *mut jsize) -> jint;

type JNICreateJavaVM = unsafe extern "system" fn(
    pvm: *mut *mut JavaVM,
    penv: *mut *mut c_void,
    args: *mut c_void,
) -> jint;

lazy_static! {
    static ref JVM_LIB: libloading::Library = {
        let full_path = format!(
            "{}{}{}",
            locate_jvm_dyn_library().expect("Could not find the jvm dynamic library"),
            MAIN_SEPARATOR,
            get_jvm_dyn_lib_file_name()
        );
        unsafe {
            libloading::Library::new(full_path).expect("Could not load the jvm dynamic library")
        }
    };
    static ref GET_CREATED_JVMS: libloading::Symbol<'static, JNIGetCreatedJavaVMs> = unsafe {
        JVM_LIB
            .get(b"JNI_GetCreatedJavaVMs")
            .expect("Could not find symbol: JNI_GetCreatedJavaVMs")
    };
    static ref CREATE_JVM: libloading::Symbol<'static, JNICreateJavaVM> = unsafe {
        JVM_LIB
            .get(b"JNI_CreateJavaVM")
            .expect("Could not find symbol: JNI_CreateJavaVM")
    };
}

pub(crate) fn get_created_java_vms(
    vm_buf: &mut Vec<*mut JavaVM>,
    buf_len: jsize,
    n_vms: *mut jsize,
) -> jint {
    unsafe { GET_CREATED_JVMS(vm_buf.as_mut_ptr(), buf_len, n_vms) }
}

pub(crate) fn create_java_vm(
    jvm: *mut *mut JavaVM,
    penv: *mut *mut c_void,
    args: *mut c_void,
) -> jint {
    unsafe { CREATE_JVM(jvm, penv, args) }
}

pub(crate) fn find_class(env: *mut JNIEnv, classname: &str) -> errors::Result<jclass> {
    unsafe {
        let cstr = utils::to_c_string(classname);
        let fc = (**env).v1_6.FindClass;
        let jc = (fc)(env, cstr);
        utils::drop_c_string(cstr);
        Ok(jc)
    }
}
