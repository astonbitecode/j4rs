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
use std::{os::raw::c_void, sync::LazyLock};
use std::sync::Mutex;

use jni_sys::{jclass, jint, jobject, jsize, JNIEnv, JavaVM, JNI_OK, JNI_TRUE};

use crate::errors::opt_to_res;
use crate::jni_utils::create_global_ref_from_local_ref;
use crate::{cache, errors, jni_utils, utils};

static MUTEX: LazyLock<Mutex<Option<J4rsAndroidJavaVM>>> = LazyLock::new(|| { Mutex::new(None) });
// Cache the classes in order to avoid classloading issues when spawning threads
static CLASSES: LazyLock<Mutex<HashMap<String, J4rsAndroidJclass>>> = LazyLock::new(|| { Mutex::new(HashMap::new()) });
static CLASSLOADER: LazyLock<Mutex<Option<J4rsAndroidClassloader>>> = LazyLock::new(|| { Mutex::new(None) });

pub fn get_created_java_vms(
    vm_buf: &mut Vec<*mut JavaVM>,
    _buf_len: jsize,
    n_vms: *mut jsize,
) -> jint {
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
            Err(error) => {
                error!("Could not get the lock for J4rsAndroidJavaVM: {:?}", error)
            }
        }
    }
    JNI_OK
}

pub(crate) fn set_java_vm(java_vm: *mut JavaVM) {
    let mut g = MUTEX.lock().unwrap();
    *g = Some(J4rsAndroidJavaVM { java_vm });
}

pub(crate) fn cache_classloader_of(env: *mut JNIEnv, obj: jobject) -> errors::Result<()> {
    unsafe {
        let classloader_instance = (opt_to_res(cache::get_jni_call_object_method())?)(
            env,
            obj,
            cache::get_get_classloader_method()?,
        );
        let mut g = CLASSLOADER.lock().unwrap();
        let classloader_instance = jni_utils::create_global_ref_from_local_ref(classloader_instance, env)?;

        *g = Some(J4rsAndroidClassloader {
            class_loader: classloader_instance,
        });
        Ok(())
    }
}

pub(crate) fn create_java_vm(
    _jvm: *mut *mut JavaVM,
    _env: *mut *mut c_void,
    _args: *mut c_void,
) -> jint {
    panic!("Cannot create Java VM in Android.")
}

// Search the class in the cache first. If not found, then call the FindClass of JNI and insert the result to the cache.
pub(crate) fn find_class(env: *mut JNIEnv, classname: &str) -> errors::Result<jclass> {
    unsafe {
        let mut add_to_cache = false;

        let found_in_cache_opt = CLASSES
            .lock()?
            .get(classname)
            .map(|j4rs_class| j4rs_class.class.clone());

        let found: errors::Result<jclass> = match found_in_cache_opt {
            Some(class) => Ok(class),
            None => {
                add_to_cache = true;
                find_class_default(env, classname)
                    .or_else(|_| find_class_using_cached_classloader(env, classname))
            }
        };

        if add_to_cache {
            let mut g = CLASSES.lock()?;
            let global = create_global_ref_from_local_ref(found?, env)?;
            g.insert(
                classname.to_string(),
                J4rsAndroidJclass {
                    class: global.clone(),
                },
            );
            Ok(global as jclass)
        } else {
            Ok(found?)
        }
    }
}

unsafe fn find_class_default(env: *mut JNIEnv, classname: &str) -> errors::Result<jclass> {
    let fc = (**env).v1_6.FindClass;
    let cstr = utils::to_c_string(classname);
    let found: jclass = (fc)(env, cstr);
    let found = do_return(env, found, classname);
    utils::drop_c_string(cstr);
    found
}

unsafe fn find_class_using_cached_classloader(
    env: *mut JNIEnv,
    classname: &str,
) -> errors::Result<jclass> {
    let g = CLASSLOADER.lock()?;
    if g.is_some() {
        let cstr = jni_utils::local_jobject_from_str(classname, env).unwrap();
        let classloader_instance = g.as_ref().unwrap().class_loader;
        let found = cache::get_jni_call_object_method().unwrap()(
            env,
            classloader_instance,
            cache::get_load_class_method().unwrap(),
            cstr,
        );

        let found = do_return(env, found, classname);
        found
    } else {
        Err(errors::J4RsError::JavaError(format!(
            "Class not found {classname}"
        )))
    }
}

fn do_return<T>(jni_env: *mut JNIEnv, to_return: T, message: &str) -> errors::Result<T> {
    unsafe {
        if (opt_to_res(cache::get_jni_exception_check())?)(jni_env) == JNI_TRUE {
            (opt_to_res(cache::get_jni_exception_clear())?)(jni_env);
            Err(errors::J4RsError::JavaError(format!(
                "Class not found {message}"
            )))
        } else {
            Ok(to_return)
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

pub(crate) struct J4rsAndroidClassloader {
    class_loader: jobject,
}

// Implementing Send and Sync is actually safe and proposed for Android to avoid classloading issues
// when creating new threads.
// https://developer.android.com/training/articles/perf-jni
unsafe impl Send for J4rsAndroidClassloader {}

unsafe impl Sync for J4rsAndroidClassloader {}
