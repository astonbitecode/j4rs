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

use std::cell::RefCell;

use crate::utils;

const MAVEN_CENTRAL: &str = "MavenCentral::https://repo.maven.apache.org/maven2";
const OSS_SNAPSHOTS: &str = "OssSnapshots::https://oss.sonatype.org/content/repositories/snapshots";

thread_local! {
    static MAVEN_SETTINGS: RefCell<MavenSettings> = RefCell::new(MavenSettings::default());
}

pub(crate) fn set_maven_settings(ms: &MavenSettings) {
    MAVEN_SETTINGS.with(|maven_settings| {
        *maven_settings.borrow_mut() = ms.clone();
    });
}

pub(crate) fn get_maven_settings() -> MavenSettings {
    MAVEN_SETTINGS.with(|maven_settings| {
        let ms = maven_settings.borrow();
        ms.clone()
    })
}

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
            base: utils::jassets_path()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("")
                .to_string(),
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
#[derive(Debug, Clone)]
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
            base: utils::jassets_path()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("")
                .to_string(),
            group: slice.first().unwrap_or(&"").to_string(),
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
    /// Convenience for creating a MavenArtifact.
    ///
    /// The `&str` should be formed like following:
    ///
    /// __group__:__id__:__version__:__qualifier__
    ///
    /// E.g:
    /// _io.github.astonbitecode:j4rs:0.5.1_
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

/// Contains Maven settings and configuration
#[derive(Debug, Clone)]
pub struct MavenSettings {
    pub(crate) repos: Vec<MavenArtifactRepo>,
}

impl MavenSettings {
    /// Creates new Maven Settings by defining additional repositories to use.
    /// The [maven central](https://repo.maven.apache.org/maven2) is always being included as a repo.
    pub fn new(repos: Vec<MavenArtifactRepo>) -> MavenSettings {
        let mut repos = repos;
        repos.push(MavenArtifactRepo::from(MAVEN_CENTRAL));
        repos.push(MavenArtifactRepo::from(OSS_SNAPSHOTS));
        MavenSettings { repos }
    }
}

impl Default for MavenSettings {
    fn default() -> Self {
        MavenSettings::new(vec![])
    }
}

/// A repository from which Java artifacts can be fetched.
#[derive(Debug, Clone)]
pub struct MavenArtifactRepo {
    pub(crate) _id: String,
    pub(crate) uri: String,
}

impl From<&[&str]> for MavenArtifactRepo {
    fn from(slice: &[&str]) -> MavenArtifactRepo {
        MavenArtifactRepo {
            _id: slice.first().unwrap_or(&"").to_string(),
            uri: slice.get(1).unwrap_or(&"").to_string(),
        }
    }
}

impl<'a> From<&'a str> for MavenArtifactRepo {
    /// Convenience for creating a MavenArtifactRepo.
    ///
    /// The `&str` should be formed like following:
    ///
    /// `id::uri`
    ///
    /// E.g:
    /// `MyAlterRepo::https://myalterrepo.io`
    fn from(string: &'a str) -> MavenArtifactRepo {
        let v: Vec<&str> = string.split("::").collect();
        MavenArtifactRepo::from(v.as_slice())
    }
}

impl From<String> for MavenArtifactRepo {
    /// Convenience for creating a MavenArtifactRepo.
    ///
    /// The `&str` should be formed like following:
    ///
    /// `id::uri`
    ///
    /// E.g:
    /// `MyAlterRepo::https://myalterrepo.io`
    fn from(string: String) -> MavenArtifactRepo {
        MavenArtifactRepo::from(string.as_str())
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

    #[test]
    fn maven_artifact_repo_from() {
        let mar = MavenArtifactRepo::from("myrepo::https://myrepo.io");
        assert_eq!(mar._id, "myrepo");
        assert_eq!(mar.uri, "https://myrepo.io");
    }
}
