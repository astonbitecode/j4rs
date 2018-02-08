extern crate fs_extra;
extern crate glob;

use glob::glob;
use std::{env, fs};
use std::error::Error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::{self, Path};

fn main() {
    // cargo:rustc-link-search=native=/usr/lib/jvm/default-runtime/jre/lib/amd64/server
    // export LD_LIBRARY_PATH=/usr/lib/jvm/default-runtime/jre/lib/amd64/server
    let out_dir = env::var("OUT_DIR").unwrap();
    let ld_library_path = get_ld_library_path();

    // Set the build environment
    println!("cargo:rustc-env=LD_LIBRARY_PATH={}", ld_library_path);
    println!("cargo:rustc-link-search=native={}", ld_library_path);
    // Generate a src file that will contain the setting of the LD_LIBRARY_PATH during runtime;
    generate_src(&out_dir, &ld_library_path);
    // Copy the needed jar files if they are available
    // (that is, if the build is done with the full source-code - not in crates.io)
    copy_jars_from_java();
    copy_jars_to_exec_directory(&out_dir);
    let _ = initialize_env(&ld_library_path).expect("Initialize Environment");
}

fn get_ld_library_path() -> String {
    // Find the JAVA_HOME
    let java_home = env::var("JAVA_HOME").unwrap_or("".to_owned());
    if java_home.is_empty() {
        panic!("JAVA_HOME is not set. j4rs cannot work without it... \
        Please make sure that Java is installed (version 1.8 at least) and the JAVA_HOME environment is set.");
    }

    let query = format!("{}/**/libjvm.so", java_home);

    let paths_vec: Vec<String> = glob(&query).unwrap()
        .filter_map(Result::ok)
        .map(|path_buf| {
            let mut pb = path_buf.clone();
            pb.pop();
            pb.to_str().unwrap().to_string()
        })
        .collect();

    if paths_vec.is_empty() {
        panic!("Could not find the libjvm.so in any subdirectory of {}", java_home);
    }

    paths_vec[0].clone()
}

fn generate_src(out_dir: &str, ld_library_path: &str) {
    let dest_path = Path::new(&out_dir).join("j4rs_env.rs");
    let mut f = File::create(&dest_path).unwrap();

    let contents = format!("
use std::env;

pub fn _init_env() {{
    env::set_var(\"LD_LIBRARY_PATH\", \"{}\");
}}
", ld_library_path);
    f.write_all(contents.as_bytes()).unwrap();
}

fn copy_jars_from_java() {
    // If the java directory exists, copy the generated jars in the `jassets` directory
    if File::open("../java").is_ok() {
        let home = env::var("CARGO_MANIFEST_DIR").unwrap();
        let jassets_path_buf = Path::new(&home).join("jassets");
        let jassets_path = jassets_path_buf.to_str().unwrap().to_owned();

        let _ = fs_extra::remove_items(vec![jassets_path.clone()].as_ref());

        let _ = fs::create_dir_all(jassets_path_buf.clone())
            .map_err(|error| panic!("Cannot create dir '{:?}': {:?}", jassets_path_buf, error));

        let jar_source_path = "../java/target/j4rs-0.1.0.jar";
        let lib_source_path = "../java/target/lib";
        let ref options = fs_extra::dir::CopyOptions::new();
        let _ = fs_extra::copy_items(vec![lib_source_path, jar_source_path].as_ref(), jassets_path, options);
    }
}

fn copy_jars_to_exec_directory(out_dir: &str) {
    let jassets_output_dir = format!("{}{}..{}..{}..{}",
                                     out_dir,
                                     path::MAIN_SEPARATOR,
                                     path::MAIN_SEPARATOR,
                                     path::MAIN_SEPARATOR,
                                     path::MAIN_SEPARATOR);

    let home = env::var("CARGO_MANIFEST_DIR").unwrap();
    let jassets_path_buf = Path::new(&home).join("jassets");
    let jassets_path = jassets_path_buf.to_str().unwrap().to_owned();

    let ref options = fs_extra::dir::CopyOptions::new();
    let _ = fs_extra::copy_items(vec![jassets_path].as_ref(), jassets_output_dir, options);
}

fn initialize_env(ld_library_path: &str) -> Result<(), J4rsBuildError> {
    let home_buf = env::home_dir().unwrap();
    let home = home_buf.to_str().unwrap_or("");
    let existing = env::var("LD_LIBRARY_PATH")?;
    if !existing.contains(ld_library_path) {
        // Add the LD_LIBRARY_PATH in the .profile
        let profile_path = format!("{}/.profile", home);
        let export_arg = format!("export LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\"", ld_library_path);
        match OpenOptions::new()
            .append(true)
            .open(profile_path) {
            Ok(mut profile_file) => {
                let to_append = format!("\n{}\n", export_arg);
                let _ = profile_file.write_all(to_append.as_bytes());
            }
            Err(error) => {
                panic!("Could not set the environment: {:?}", error);
            }
        };
        println!("cargo:warning=The contents of $HOME/.profile changed, by adding the libjni location in the LD_LIBRARY_PATH env variable.\
         This is done becaust the jni shared library is needed by j4rs. In order to use j4rs in this session, please source the $HOME/.profile, or log out and log in.");
    }
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
