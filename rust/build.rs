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

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::{env, fs};

use fs_extra::dir::CopyOptions;

// This is the version of the jar that should be used
const VERSION: &str = "0.22.0-SNAPSHOT";
const JAVA_FX_VERSION: &str = "21.0.2";

fn main() -> Result<(), J4rsBuildError> {
    // ensure build.rs is not rerun when there are no `rerun-if-*` printed
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR")?;
    let source_jar_location = PathBuf::from(format!(
        "../java/target/j4rs-{VERSION}-jar-with-dependencies.jar"
    ));
    if source_jar_location.exists() {
        println!(
            "cargo:rerun-if-changed={}",
            source_jar_location.to_string_lossy()
        );
    }
    generate_src(&out_dir)?;

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        // skip the rest of setup on android
        return Ok(());
    }

    // Copy the needed jar files if they are available
    // (that is, if the build is done with the full source-code - not in crates.io)
    copy_jars_from_java(&source_jar_location)?;

    copy_jars_to_target_directory(&out_dir)?;

    Ok(())
}

fn generate_src(out_dir: &str) -> Result<(), J4rsBuildError> {
    let dest_path = Path::new(&out_dir).join("j4rs_init.rs");
    let contents = format!(
        "
pub(crate) fn j4rs_version() -> &'static str {{
    \"{VERSION}\"
}}

pub(crate) fn java_fx_version() -> &'static str {{
    \"{JAVA_FX_VERSION}\"
}}
"
    );
    std::fs::write(dest_path, contents)?;
    Ok(())
}

// Copies the jars from the `java` directory to the source directory of rust.
fn copy_jars_from_java(jar_source_path: &Path) -> Result<(), J4rsBuildError> {
    if jar_source_path.exists() {
        // Find the destination file
        let jassets_path = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("jassets");

        let destination_jar_file =
            Path::new(&jassets_path).join(format!("j4rs-{VERSION}-jar-with-dependencies.jar"));

        // Copy only if the files are not the same
        let do_copy = if destination_jar_file.exists() {
            !are_same_files(jar_source_path, &destination_jar_file).unwrap_or(true)
        } else {
            true
        };

        if do_copy {
            fs_extra::remove_items(&[&jassets_path])?;

            fs::create_dir_all(&jassets_path)
                .map_err(|error| panic!("Cannot create dir '{jassets_path:?}': {error:?}"))
                .ok();

            fs_extra::copy_items(&[jar_source_path], jassets_path, &CopyOptions::new())?;
        }
    }
    Ok(())
}

fn are_same_files(path1: &Path, path2: &Path) -> Result<bool, J4rsBuildError> {
    Ok(std::fs::read(path1)? == std::fs::read(path2)?)
}

// Copies the jars to CARGO_TARGET_DIR/*/jassets
fn copy_jars_to_target_directory(out_dir: &str) -> Result<(), J4rsBuildError> {
    let mut target_dir = PathBuf::from(out_dir);
    target_dir.pop();
    target_dir.pop();
    target_dir.pop();

    let jassets_path = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("jassets");
    let jassets_jar_file = jassets_path.join(format!("j4rs-{VERSION}-jar-with-dependencies.jar"));

    let jassets_output_file = target_dir
        .join("jassets")
        .join(format!("j4rs-{VERSION}-jar-with-dependencies.jar"));

    // Delete the target jassets and copy only if the files are not the same
    let do_copy = if jassets_jar_file.exists() && jassets_output_file.exists() {
        !are_same_files(&jassets_jar_file, &jassets_output_file).unwrap_or(true)
    } else {
        true
    };

    if do_copy {
        fs_extra::remove_items(&[target_dir.join("jassets")])?;
        fs_extra::copy_items(&[jassets_path], target_dir, &CopyOptions::new())?;
    }

    Ok(())
}

#[derive(Debug)]
struct J4rsBuildError {
    description: String,
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
        J4rsBuildError {
            description: format!("{:?}", err),
        }
    }
}

impl From<std::io::Error> for J4rsBuildError {
    fn from(err: std::io::Error) -> J4rsBuildError {
        J4rsBuildError {
            description: format!("{:?}", err),
        }
    }
}

impl From<fs_extra::error::Error> for J4rsBuildError {
    fn from(err: fs_extra::error::Error) -> J4rsBuildError {
        J4rsBuildError {
            description: format!("{:?}", err),
        }
    }
}
