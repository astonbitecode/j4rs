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