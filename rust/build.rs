extern crate dirs;
extern crate fs_extra;
extern crate glob;

use std::{env, fs};
use std::error::Error;
use std::fmt;
#[allow(unused_imports)]
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use glob::glob;

const VERSION: &'static str = "0.4.0-SNAPSHOT";

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let target_os_res = env::var("CARGO_CFG_TARGET_OS");
    let target_os = target_os_res.as_ref().map(|x| &**x).unwrap_or("unknown");
    if target_os == "android" {
        generate_src(&out_dir);
        return;
    }

    let jvm_dyn_lib_file_name = if target_os == "windows" {
        "jvm.dll"
    } else {
        "libjvm.*"
    };
    let ld_library_path = get_ld_library_path(jvm_dyn_lib_file_name);

    // Set the build environment
    if target_os == "windows" {
        println!("cargo:rustc-env=PATH={};%PATH%", ld_library_path);
        let jvm_lib = get_ld_library_path("jvm.lib");
        println!("cargo:rustc-link-search={}", jvm_lib);
    } else if target_os == "macos" {
        let ld = env::var("DYLD_LIBRARY_PATH").unwrap_or("".to_string());
        println!("cargo:rustc-env=DYLD_LIBRARY_PATH={}:{}", ld_library_path, ld);
        println!("cargo:rustc-link-search={}", ld_library_path);
    } else {
        let ld = env::var("LD_LIBRARY_PATH").unwrap_or("".to_string());
        println!("cargo:rustc-env=LD_LIBRARY_PATH={}:{}", ld_library_path, ld);
        println!("cargo:rustc-link-search={}", ld_library_path);
    }
    // Copy the needed jar files if they are available
    // (that is, if the build is done with the full source-code - not in crates.io)
    copy_jars_from_java();
    let _ = copy_jars_to_exec_directory(&out_dir);
    initialize_env(&ld_library_path, target_os).expect("Initialize Environment");
    generate_src(&out_dir);
}

// Finds and returns the directory that contains the libjvm library
fn get_ld_library_path(lib_file_name: &str) -> String {
    // Find the JAVA_HOME
    let java_home = env::var("JAVA_HOME").unwrap_or("".to_owned());
    if java_home.is_empty() {
        panic!("JAVA_HOME is not set. j4rs cannot work without it... \
        Please make sure that Java is installed (version 1.8 at least) and the JAVA_HOME environment is set.");
    }

    let query = format!("{}/**/{}", java_home, lib_file_name);

    let paths_vec: Vec<String> = glob(&query).unwrap()
        .filter_map(Result::ok)
        .map(|path_buf| {
            let mut pb = path_buf.clone();
            pb.pop();
            pb.to_str().unwrap().to_string()
        })
        .collect();

    if paths_vec.is_empty() {
        let name = if cfg!(windows) {
            "jvm.lib"
        } else {
            "libjvm"
        };
        panic!("Could not find the {} in any subdirectory of {}", name, java_home);
    }

    paths_vec[0].clone()
}

fn generate_src(out_dir: &str) {
    let dest_path = Path::new(&out_dir).join("j4rs_init.rs");
    let mut f = File::create(&dest_path).unwrap();

    let contents = format!(
        "
fn j4rs_version() -> &'static str {{
    \"{}\"
}}
", VERSION);

    f.write_all(contents.as_bytes()).unwrap();
}

// Copies the jars from the `java` directory to the source directory of rust.
fn copy_jars_from_java() {
    // If the java directory exists, copy the generated jars in the `jassets` directory
    let jar_source_path = format!("../java/target/j4rs-{}-jar-with-dependencies.jar", VERSION);
    if File::open(&jar_source_path).is_ok() {
        let home = env::var("CARGO_MANIFEST_DIR").unwrap();
        let jassets_path_buf = Path::new(&home).join("jassets");
        let jassets_path = jassets_path_buf.to_str().unwrap().to_owned();

        let _ = fs_extra::remove_items(vec![jassets_path.clone()].as_ref());

        let _ = fs::create_dir_all(jassets_path_buf.clone())
            .map_err(|error| panic!("Cannot create dir '{:?}': {:?}", jassets_path_buf, error));

        let ref options = fs_extra::dir::CopyOptions::new();
        let _ = fs_extra::copy_items(vec![jar_source_path].as_ref(), jassets_path, options);
    }
}

// Copies the jars to and returns the PathBuf of the exec directory.
fn copy_jars_to_exec_directory(out_dir: &str) -> PathBuf {
    let mut exec_dir_path_buf = PathBuf::from(out_dir);
    exec_dir_path_buf.pop();
    exec_dir_path_buf.pop();
    exec_dir_path_buf.pop();

    let jassets_output = exec_dir_path_buf.clone();
    let jassets_output_dir = jassets_output.to_str().unwrap();


    let home = env::var("CARGO_MANIFEST_DIR").unwrap();
    let jassets_path_buf = Path::new(&home).join("jassets");
    let jassets_path = jassets_path_buf.to_str().unwrap().to_owned();

    let ref options = fs_extra::dir::CopyOptions::new();
    let _ = fs_extra::copy_items(vec![jassets_path].as_ref(), jassets_output_dir, options);
    exec_dir_path_buf
}

fn initialize_env(ld_library_path: &str, target_os: &str) -> Result<(), J4rsBuildError> {
    match target_os {
        "macos" => initialize_env_macos(ld_library_path),
        "linux" => initialize_env_linux(ld_library_path),
        "windows" => initialize_env_windows(ld_library_path),
        other => {
            println!("cargo:warning=Cannot initialize the environment for target os {}", other);
            Ok(())
        }
    }
}

fn initialize_env_macos(ld_library_path: &str) -> Result<(), J4rsBuildError> {
    let existing = env::var("DYLD_LIBRARY_PATH").unwrap_or("".to_owned());
    if !existing.contains(ld_library_path) {
        println!("cargo:warning=Please add to the DYLD_LIBRARY_PATH env the following: {}", ld_library_path);
    }
    Ok(())
}

// Appends the jni lib directory in the case that it is not contained in the LD_LIBRARY_PATH.
// Appends the entry in the $CARGO_HOME/env.
fn initialize_env_linux(ld_library_path: &str) -> Result<(), J4rsBuildError> {
    let existing = env::var("LD_LIBRARY_PATH")?;
    let chome = env::var("CARGO_HOME").unwrap_or("".to_owned());

    if chome.is_empty() {
        println!("cargo:warning=Please add to the LD_LIBRARY_PATH env the following: {}", ld_library_path);
    } else {
        let env_file_path = format!("{}/env", chome);
        let export_arg = format!("export LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\"", ld_library_path);

        let exists_in_profile = {
            let mut f = File::open(&env_file_path)?;
            let mut buffer = String::new();
            f.read_to_string(&mut buffer)?;
            buffer.contains(&export_arg)
        };

        if !existing.contains(ld_library_path) && !exists_in_profile {
            // Add the LD_LIBRARY_PATH in the env
            match OpenOptions::new()
                .append(true)
                .open(env_file_path) {
                Ok(mut env_file) => {
                    let to_append = format!("\n{}\n", export_arg);
                    let _ = env_file.write_all(to_append.as_bytes());
                }
                Err(error) => {
                    panic!("Could not set the environment: {:?}", error);
                }
            };
            println!("cargo:warning=The contents of {}/env changed, by adding the libjvm location in the LD_LIBRARY_PATH env variable.\
         This is done because the jvm shared library is needed to run Java natively. In order to use j4rs in this session, please source the {}/env changed, or log out and log in.", chome, chome);
        }
    }
    Ok(())
}

fn initialize_env_windows(_: &str) -> Result<(), J4rsBuildError> {
    Ok(())
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
