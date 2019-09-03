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

use std::{self, fs, str};
use std::ffi::{CStr, CString};
use std::path::PathBuf;

use fs_extra::dir::get_dir_content;
use libc::c_char;

use crate::{api, errors, InvocationArg};

pub fn to_rust_string(pointer: *const c_char) -> String {
    let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
    str::from_utf8(slice).unwrap().to_string()
}

pub fn to_java_string(string: &str) -> *mut c_char {
    let cs = CString::new(string.as_bytes()).unwrap();
    cs.into_raw()
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
    Ok(pb
        .to_str()
        .unwrap_or("./deps/").to_owned())
}

pub(crate) fn jassets_path() -> errors::Result<PathBuf> {
    let pb_opt = {
        let guard = api::JASSETS_PATH.lock()?;
        guard.clone()
    };
    match pb_opt {
        Some(pb) => Ok(pb),
        None => default_jassets_path(),
    }
}

pub(crate) fn default_jassets_path() -> errors::Result<PathBuf> {
    let mut jassets_path = std::env::current_exe()?;
    let mut tmp_vec = Vec::new();

    while tmp_vec.is_empty() {
        jassets_path.pop();
        tmp_vec = get_dir_content(&jassets_path)?.directories.into_iter().filter(|path| path.ends_with("jassets")).collect();
    }

    jassets_path.push("jassets");
    Ok(jassets_path)
}


pub(crate) fn find_j4rs_dynamic_libraries_names() -> errors::Result<Vec<String>> {
    let entries: Vec<String> = find_j4rs_dynamic_libraries_dir_entries()?.iter()
        .map(|entry| entry
            .file_name()
            .to_str()
            .unwrap()
            .to_owned())
        .collect();

    Ok(entries)
}

pub(crate) fn find_j4rs_dynamic_libraries_paths() -> errors::Result<Vec<String>> {
    let entries: Vec<String> = find_j4rs_dynamic_libraries_dir_entries()?.iter()
        .map(|entry| entry
            .path()
            .to_str()
            .unwrap()
            .to_owned())
        .collect();

    Ok(entries)
}

fn find_j4rs_dynamic_libraries_dir_entries() -> errors::Result<Vec<fs::DirEntry>> {
    let v: Vec<fs::DirEntry> = fs::read_dir(deps_dir()?)?
        .filter(|entry| {
            entry.is_ok()
        })
        .filter(|entry| {
            let entry = entry.as_ref().unwrap();
            let file_name = entry.file_name();
            let file_name = file_name.to_str().unwrap();
            file_name.contains("j4rs") && (
                file_name.contains(".so") ||
                    file_name.contains(".dll") ||
                    file_name.contains(".dylib"))
        })
        .map(|entry| entry.unwrap())
        .collect();

    Ok(v)
}

pub(crate) fn primitive_of(inv_arg: &InvocationArg) -> Option<String> {
    match get_class_name(inv_arg) {
        "java.lang.Boolean" => Some("bool".to_string()),
        "java.lang.Byte" => Some("byte".to_string()),
        "java.lang.Short" => Some("short".to_string()),
        "java.lang.Integer" => Some("int".to_string()),
        "java.lang.Long" => Some("long".to_string()),
        "java.lang.Float" => Some("float".to_string()),
        "java.lang.Double" => Some("double".to_string()),
        "java.lang.Character" => Some("char".to_string()),
        "void" => Some("void".to_string()),
        _ => None,
    }
}

pub(crate) fn get_class_name(inv_arg: &InvocationArg) -> &str {
    let class_name = match inv_arg {
        &InvocationArg::Java { instance: _, ref class_name, arg_from: _ } => class_name,
        &InvocationArg::Rust { json: _, ref class_name, arg_from: _ } => class_name,
    };
    class_name.as_ref()
}

#[cfg(test)]
mod utils_unit_tests {
    use super::*;

    #[test]
    fn get_class_name_test() {
        assert!(get_class_name(&InvocationArg::from(false)) == "java.lang.Boolean");
    }

    #[test]
    fn primitive_of_test() {
        assert!(primitive_of(&InvocationArg::from(false)) == Some("bool".to_string()));
        assert!(primitive_of(&InvocationArg::from(1_i8)) == Some("byte".to_string()));
        assert!(primitive_of(&InvocationArg::from(1_i16)) == Some("short".to_string()));
        assert!(primitive_of(&InvocationArg::from(1_32)) == Some("int".to_string()));
        assert!(primitive_of(&InvocationArg::from(1_i64)) == Some("long".to_string()));
        assert!(primitive_of(&InvocationArg::from(0.1_f32)) == Some("float".to_string()));
        assert!(primitive_of(&InvocationArg::from(0.1_f64)) == Some("double".to_string()));
        assert!(primitive_of(&InvocationArg::from('c')) == Some("char".to_string()));
        assert!(primitive_of(&InvocationArg::from(())) == Some("void".to_string()));
    }
}