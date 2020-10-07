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
extern crate dirs;
extern crate fs_extra;

use std::{env, fs};
use std::error::Error;
use std::fmt;
#[allow(unused_imports)]
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use java_locator;
use sha2::{Digest, Sha256};

// This is the version of the jar that should be used
const VERSION: &'static str = "0.13.0-java7";

fn main() -> Result<(), J4rsBuildError> {
    let out_dir = env::var("OUT_DIR")?;
    let source_jar_location = format!("../java/target/j4rs-{}-jar-with-dependencies.jar", VERSION);
    if File::open(&source_jar_location).is_ok() {
        println!("cargo:rerun-if-changed={}", source_jar_location);
    }

    let target_os_res = env::var("CARGO_CFG_TARGET_OS");
    let target_os = target_os_res.as_ref().map(|x| &**x).unwrap_or("unknown");
    if target_os == "android" {
        generate_src(&out_dir)?;
        return Ok(());
    }

    // Copy the needed jar files if they are available
    // (that is, if the build is done with the full source-code - not in crates.io)
    copy_jars_from_java(&source_jar_location)?;
    let _ = copy_jars_to_exec_directory(&out_dir)?;
    generate_src(&out_dir)?;

    Ok(())
}

fn generate_src(out_dir: &str) -> Result<(), J4rsBuildError> {
    let dest_path = Path::new(&out_dir).join("j4rs_init.rs");
    let mut f = File::create(&dest_path)?;

    let contents = format!(
        "
fn j4rs_version() -> &'static str {{
    \"{}\"
}}
", VERSION);

    f.write_all(contents.as_bytes())?;
    Ok(())
}

// Copies the jars from the `java` directory to the source directory of rust.
fn copy_jars_from_java(jar_source_path: &str) -> Result<(), J4rsBuildError> {
    if let Ok(mut source_jar_file) = File::open(&jar_source_path) {
        // Find the destination file
        let home = env::var("CARGO_MANIFEST_DIR")?;
        let jassets_path_buf = Path::new(&home).join("jassets");
        let jassets_path = jassets_path_buf.to_str().unwrap().to_owned();

        let destination_jar_file_res = {
            let djpb = Path::new(&jassets_path).join(format!("j4rs-{}-jar-with-dependencies.jar", VERSION));
            File::open(djpb)
        };

        // Copy only if the files are not the same
        let do_copy = if destination_jar_file_res.is_ok() {
            let mut destination_jar_file = destination_jar_file_res.unwrap();
            !are_same_files(&mut source_jar_file, &mut destination_jar_file).unwrap_or(true)
        } else { true };

        if do_copy {
            fs_extra::remove_items(vec![jassets_path.clone()].as_ref())?;

            let _ = fs::create_dir_all(jassets_path_buf.clone())
                .map_err(|error| panic!("Cannot create dir '{:?}': {:?}", jassets_path_buf, error));

            let ref options = fs_extra::dir::CopyOptions::new();
            let _ = fs_extra::copy_items(vec![jar_source_path].as_ref(), jassets_path, options)?;
        }
    }
    Ok(())
}

fn are_same_files(f1: &mut File, f2: &mut File) -> Result<bool, J4rsBuildError> {
    let mut buffer1: Vec<u8> = Vec::new();
    let mut hasher1 = Sha256::new();
    let mut buffer2: Vec<u8> = Vec::new();
    let mut hasher2 = Sha256::new();

    f1.read_to_end(&mut buffer1)?;
    hasher1.update(&buffer1);
    let hash1 = hasher1.finalize();

    f2.read_to_end(&mut buffer2)?;
    hasher2.update(&buffer2);
    let hash2 = hasher2.finalize();

    Ok(hash1 == hash2)
}

// Copies the jars to and returns the PathBuf of the exec directory.
fn copy_jars_to_exec_directory(out_dir: &str) -> Result<PathBuf, J4rsBuildError> {
    let mut exec_dir_path_buf = PathBuf::from(out_dir);
    exec_dir_path_buf.pop();
    exec_dir_path_buf.pop();
    exec_dir_path_buf.pop();

    let jassets_output = exec_dir_path_buf.clone();
    let jassets_output_dir = jassets_output.to_str().unwrap();


    let home = env::var("CARGO_MANIFEST_DIR")?;
    let jassets_path_buf = Path::new(&home).join("jassets");
    let jassets_path = jassets_path_buf.to_str().unwrap().to_owned();
    fs_extra::remove_items(vec![format!("{}/jassets", jassets_output_dir)].as_ref())?;

    let ref options = fs_extra::dir::CopyOptions::new();
    let _ = fs_extra::copy_items(vec![jassets_path].as_ref(), jassets_output_dir, options)?;
    Ok(exec_dir_path_buf)
}

#[derive(Debug)]
struct J4rsBuildError {
    description: String
}

impl fmt::Display for J4rsBuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl Error for J4rsBuildError {
    fn description(&self) -> &str {
        self.description.as_str()
    }
}

impl From<std::env::VarError> for J4rsBuildError {
    fn from(err: std::env::VarError) -> J4rsBuildError {
        J4rsBuildError { description: format!("{:?}", err) }
    }
}

impl From<std::io::Error> for J4rsBuildError {
    fn from(err: std::io::Error) -> J4rsBuildError {
        J4rsBuildError { description: format!("{:?}", err) }
    }
}

impl From<java_locator::errors::JavaLocatorError> for J4rsBuildError {
    fn from(err: java_locator::errors::JavaLocatorError) -> J4rsBuildError {
        J4rsBuildError { description: format!("{:?}", err) }
    }
}

impl From<fs_extra::error::Error> for J4rsBuildError {
    fn from(err: fs_extra::error::Error) -> J4rsBuildError {
        J4rsBuildError { description: format!("{:?}", err) }
    }
}