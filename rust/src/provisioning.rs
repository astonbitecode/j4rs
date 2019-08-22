// Copyright 2019 astonbitecode
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

use crate::utils;
use std::path::PathBuf;

/// Marker trait to be used for deploying artifacts.
pub trait JavaArtifact {}

/// Represents a Jar artifact that resides in the local storage.
/// It can be deployed in order to be loaded and used by j4rs by calling the `JVM::deploy_artifact` method.
#[derive(Debug)]
pub struct LocalJarArtifact {
    pub(crate) base: String,
    pub(crate) path: String,
}

impl LocalJarArtifact {
    /// Creates a new LocalJarArtifact.
    /// path is the location of the jar file in the local storage
    pub fn new(path: &str) -> LocalJarArtifact {
        LocalJarArtifact {
            base: utils::jassets_path().unwrap_or(PathBuf::new()).to_str().unwrap_or("").to_string(),
            path: path.to_string(),
        }
    }
}

impl JavaArtifact for LocalJarArtifact {}

impl<'a> From<&'a str> for LocalJarArtifact {
    fn from(string: &'a str) -> LocalJarArtifact {
        LocalJarArtifact::new(string)
    }
}

impl From<String> for LocalJarArtifact {
    fn from(string: String) -> LocalJarArtifact {
        LocalJarArtifact::new(&string)
    }
}

/// Represents an Artifact that can be fetched by a remote Maven repository.
/// It can loaded and used by j4rs by calling the `JVM::deploy_artifact` method.
#[derive(Debug)]
pub struct MavenArtifact {
    pub(crate) base: String,
    pub(crate) group: String,
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) qualifier: String,
}

impl JavaArtifact for MavenArtifact {}

impl From<&[&str]> for MavenArtifact {

    fn from(slice: &[&str]) -> MavenArtifact {
        MavenArtifact {
            base: utils::jassets_path().unwrap_or(PathBuf::new()).to_str().unwrap_or("").to_string(),
            group: slice.get(0).unwrap_or(&"").to_string(),
            id: slice.get(1).unwrap_or(&"").to_string(),
            version: slice.get(2).unwrap_or(&"").to_string(),
            qualifier: slice.get(3).unwrap_or(&"").to_string(),
        }
    }
}

impl From<Vec<&str>> for MavenArtifact {
    fn from(v: Vec<&str>) -> MavenArtifact {
        MavenArtifact::from(v.as_slice())
    }
}

impl From<&Vec<&str>> for MavenArtifact {
    fn from(v: &Vec<&str>) -> MavenArtifact {
        MavenArtifact::from(v.as_slice())
    }
}

impl<'a> From<&'a str> for MavenArtifact {
    fn from(string: &'a str) -> MavenArtifact {
        let v: Vec<&str> = string.split(':').collect();
        MavenArtifact::from(v.as_slice())
    }
}

impl From<String> for MavenArtifact {
    /// Convenience for creating a MavenArtifact.
    ///
    /// The `&str` should be formed like following:
    ///
    /// __group__:__id__:__version__:__qualifier__
    ///
    /// E.g:
    /// _io.github.astonbitecode:j4rs:0.5.1_
    fn from(string: String) -> MavenArtifact {
        let v: Vec<&str> = string.split(':').collect();
        MavenArtifact::from(v.as_slice())
    }
}

#[cfg(test)]
mod provisioning_unit_tests {
    use super::*;

    #[test]
    fn maven_artifact_from() {
        let ma1 = MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1");
        assert_eq!(ma1.group, "io.github.astonbitecode");
        assert_eq!(ma1.id, "j4rs");
        assert_eq!(ma1.version, "0.5.1");
        assert_eq!(ma1.qualifier, "");

        let ma2 = MavenArtifact::from("io.github.astonbitecode:j4rs:0.5.1".to_string());
        assert_eq!(ma2.group, "io.github.astonbitecode");
        assert_eq!(ma2.id, "j4rs");
        assert_eq!(ma2.version, "0.5.1");
        assert_eq!(ma2.qualifier, "");

        let ma3 = MavenArtifact::from(&vec!["io.github.astonbitecode", "j4rs", "0.5.1"]);
        assert_eq!(ma3.group, "io.github.astonbitecode");
        assert_eq!(ma3.id, "j4rs");
        assert_eq!(ma3.version, "0.5.1");
        assert_eq!(ma3.qualifier, "");
    }

}