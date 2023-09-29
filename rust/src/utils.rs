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

use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::{self, env, fs, str};

use cesu8::{from_java_cesu8, to_java_cesu8};
use dunce::canonicalize;
use fs_extra::dir::get_dir_content;
use libc::{self, c_char};

use crate::api::{
    PRIMITIVE_BOOLEAN, PRIMITIVE_BYTE, PRIMITIVE_CHAR, PRIMITIVE_DOUBLE, PRIMITIVE_FLOAT,
    PRIMITIVE_INT, PRIMITIVE_LONG, PRIMITIVE_SHORT,
};
use crate::{cache, errors, InvocationArg, JavaClass};

pub fn to_rust_string(pointer: *const c_char) -> String {
    let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
    from_java_cesu8(slice).unwrap().to_string()
}

pub fn to_c_string(string: &str) -> *mut c_char {
    let cs = CString::new(string.as_bytes()).unwrap();
    cs.into_raw()
}

pub fn to_c_string_struct(string: &str) -> CString {
    let enc = to_java_cesu8(string).into_owned();
    unsafe { CString::from_vec_unchecked(enc) }
}

pub fn drop_c_string(ptr: *mut c_char) {
    let _ = unsafe { CString::from_raw(ptr) };
}

#[cfg(not(target_os = "windows"))]
pub fn classpath_sep() -> &'static str {
    ":"
}

#[cfg(target_os = "windows")]
pub fn classpath_sep() -> &'static str {
    ";"
}

pub fn java_library_path() -> errors::Result<String> {
    let default = format!("-Djava.library.path={}", deps_dir()?);
    if cfg!(windows) {
        Ok(default)
    } else {
        Ok(format!("{}:/usr/lib:/lib", default))
    }
}

pub(crate) fn deps_dir() -> errors::Result<String> {
    let mut pb = jassets_path()?;
    pb.pop();
    pb.push("deps");
    Ok(pb.to_str().unwrap_or("./deps/").to_owned())
}

pub(crate) fn jassets_path() -> errors::Result<PathBuf> {
    let pb_opt = {
        let guard = cache::JASSETS_PATH.lock()?;
        guard.clone()
    };
    match pb_opt {
        Some(pb) => Ok(pb),
        None => default_jassets_path(),
    }
}

pub(crate) fn default_jassets_path() -> errors::Result<PathBuf> {
    let is_build_script = env::var("OUT_DIR").is_ok();

    let mut jassets_path = if is_build_script {
        PathBuf::from(env::var("OUT_DIR")?)
    } else {
        env::current_exe()?
    };
    jassets_path = canonicalize(jassets_path)?;

    let mut tmp_vec = Vec::new();

    while tmp_vec.is_empty() {
        jassets_path.pop();
        tmp_vec = get_dir_content(&jassets_path)?
            .directories
            .into_iter()
            .filter(|path| path.ends_with("jassets"))
            .collect();
    }

    jassets_path.push("jassets");
    Ok(jassets_path)
}

pub(crate) fn find_j4rs_dynamic_libraries_names() -> errors::Result<Vec<String>> {
    let entries: Vec<String> = find_j4rs_dynamic_libraries_dir_entries()?
        .iter()
        .map(|entry| entry.file_name().to_str().unwrap().to_owned())
        .collect();

    Ok(entries)
}

pub(crate) fn find_j4rs_dynamic_libraries_paths() -> errors::Result<Vec<String>> {
    let entries: Vec<String> = find_j4rs_dynamic_libraries_dir_entries()?
        .iter()
        .map(|entry| entry.path().to_str().unwrap().to_owned())
        .collect();

    Ok(entries)
}

fn find_j4rs_dynamic_libraries_dir_entries() -> errors::Result<Vec<fs::DirEntry>> {
    let v: Vec<fs::DirEntry> = fs::read_dir(deps_dir()?)?
        .filter(|entry| entry.is_ok())
        .filter(|entry| {
            let entry = entry.as_ref().unwrap();
            let file_name = entry.file_name();
            let file_name = file_name.to_str().unwrap();
            file_name.contains("j4rs")
                && (file_name.contains(".so")
                    || file_name.contains(".dll")
                    || file_name.contains(".dylib"))
        })
        .map(|entry| entry.unwrap())
        .collect();

    Ok(v)
}

pub(crate) fn primitive_of(inv_arg: &InvocationArg) -> Option<String> {
    match get_class_name(inv_arg).into() {
        JavaClass::Boolean => Some(PRIMITIVE_BOOLEAN.to_string()),
        JavaClass::Byte => Some(PRIMITIVE_BYTE.to_string()),
        JavaClass::Short => Some(PRIMITIVE_SHORT.to_string()),
        JavaClass::Integer => Some(PRIMITIVE_INT.to_string()),
        JavaClass::Long => Some(PRIMITIVE_LONG.to_string()),
        JavaClass::Float => Some(PRIMITIVE_FLOAT.to_string()),
        JavaClass::Double => Some(PRIMITIVE_DOUBLE.to_string()),
        JavaClass::Character => Some(PRIMITIVE_CHAR.to_string()),
        JavaClass::Void => Some("void".to_string()),
        _ => None,
    }
}

pub(crate) fn get_class_name(inv_arg: &InvocationArg) -> &str {
    let class_name = match inv_arg {
        &InvocationArg::Java {
            instance: _,
            ref class_name,
            serialized: _,
        } => class_name,
        &InvocationArg::Rust {
            json: _,
            ref class_name,
            serialized: _,
        } => class_name,
        &InvocationArg::RustBasic {
            instance: _,
            ref class_name,
            serialized: _,
        } => class_name,
    };
    class_name.as_ref()
}

#[cfg(test)]
mod utils_unit_tests {
    use std::convert::TryFrom;

    use crate::JvmBuilder;

    use super::*;

    #[test]
    fn get_class_name_test() {
        let _jvm = JvmBuilder::new().build().unwrap();
        assert!(get_class_name(&InvocationArg::try_from(false).unwrap()) == "java.lang.Boolean");
    }

    #[test]
    fn primitive_of_test() {
        let _jvm = JvmBuilder::new().build().unwrap();
        assert!(
            primitive_of(&InvocationArg::try_from(false).unwrap()) == Some("boolean".to_string())
        );
        assert!(primitive_of(&InvocationArg::try_from(1_i8).unwrap()) == Some("byte".to_string()));
        assert!(
            primitive_of(&InvocationArg::try_from(1_i16).unwrap()) == Some("short".to_string())
        );
        assert!(primitive_of(&InvocationArg::try_from(1_32).unwrap()) == Some("int".to_string()));
        assert!(primitive_of(&InvocationArg::try_from(1_i64).unwrap()) == Some("long".to_string()));
        assert!(
            primitive_of(&InvocationArg::try_from(0.1_f32).unwrap()) == Some("float".to_string())
        );
        assert!(
            primitive_of(&InvocationArg::try_from(0.1_f64).unwrap()) == Some("double".to_string())
        );
        assert!(primitive_of(&InvocationArg::try_from('c').unwrap()) == Some("char".to_string()));
        assert!(primitive_of(&InvocationArg::try_from(()).unwrap()) == Some("void".to_string()));
    }
}
