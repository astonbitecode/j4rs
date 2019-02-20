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
use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::Mutex;

use jni_sys::{
    JavaVM,
    jclass,
    jint,
    JNI_OK,
    JNIEnv,
    jsize,
};

use crate::api::create_global_ref_from_local_ref;
use crate::utils;

lazy_static! {
    static ref MUTEX: Mutex<Option<J4rsAndroidJavaVM>> = Mutex::new(None);
    // Cache the classes in order to avoid classloading issues when spawning threads
    static ref CLASSES: Mutex<HashMap<String, J4rsAndroidJclass>> = Mutex::new(HashMap::new());
}

pub fn get_created_java_vms(vm_buf: &mut Vec<*mut JavaVM>, _buf_len: jsize, n_vms: *mut jsize) -> jint {
    unsafe {
        match MUTEX.lock() {
            Ok(g) => {
                if g.is_some() {
                    *n_vms = 1;
                    *vm_buf = vec![g.as_ref().unwrap().java_vm.clone()];
                } else {
                    *n_vms = 0;
                    *vm_buf = Vec::new();
                };
            }
            Err(error) => { error!("Could not get the lock for J4rsAndroidJavaVM: {:?}", error) }
        }
    }
    JNI_OK
}

pub(crate) fn set_java_vm(java_vm: *mut JavaVM) {
    let mut g = MUTEX.lock().unwrap();
    *g = Some(J4rsAndroidJavaVM { java_vm });
}

pub(crate) fn create_java_vm(
    _pvm: *mut *mut JavaVM,
    _penv: *mut *mut c_void,
    _args: *mut c_void,
) -> jint { panic!("Cannot create Java VM in Android.") }

// Search the class in the cache first. If not found, then call the FindClass of JNI and insert the result to the cache.
pub(crate) fn find_class(env: *mut JNIEnv, classname: &str) -> jclass {
    unsafe {
        let mut add_to_cache = false;
        let found = match CLASSES.lock() {
            Ok(mut g) => {
                match g.get(classname) {
                    Some(j4rs_class) => {
                        Some(j4rs_class.class.clone())
                    }
                    None => {
                        ((**env).FindClass)
                            .map(|fc| {
                                let found: jclass = (fc)(
                                    env,
                                    utils::to_java_string(classname),
                                );
                                add_to_cache = true;
                                found
                            })
                    }
                }
            }
            Err(error) => {
                error!("Could not get the lock for the jclass cache: {:?}", error);
                None
            }
        };

        let to_ret = found.expect("Error while calling find_class");
        if add_to_cache {
            let global = create_global_ref_from_local_ref(to_ret, env)
                .expect(&format!("Could not create global ref of jclass {}", classname));
            CLASSES.lock()
                .expect("Could not get mutable lock to add to the jclass cache")
                .insert(classname.to_string(), J4rsAndroidJclass { class: global.clone() });
            global as jclass
        } else {
            to_ret
        }
    }
}

pub(crate) struct J4rsAndroidJavaVM {
    java_vm: *mut JavaVM,
}

// Implementing Send and Sync is actually safe and proposed for Android
// https://developer.android.com/training/articles/perf-jni
unsafe impl Send for J4rsAndroidJavaVM {}

unsafe impl Sync for J4rsAndroidJavaVM {}

pub(crate) struct J4rsAndroidJclass {
    class: jclass,
}

// Implementing Send and Sync is actually safe and proposed for Android to avoid classloading issues
// when creating new threads.
// https://developer.android.com/training/articles/perf-jni
unsafe impl Send for J4rsAndroidJclass {}

unsafe impl Sync for J4rsAndroidJclass {}