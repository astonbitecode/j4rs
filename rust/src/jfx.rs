// Copyright 2020 astonbitecode
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
use std::env;

use crate::{InstanceReceiver, Jvm, MavenArtifact};
use crate::errors;
use crate::errors::J4RsError;

pub trait JavaFxSupport {
    /// Triggers the start of a JavaFX application.
    /// When the JavaFX application starts, the `InstanceReceiver` channel will receive an Instance of `javafx.stage.Stage`.
    ///
    /// The UI may start being built using the provided `Stage`
    fn start_javafx_app(&self) -> errors::Result<InstanceReceiver>;
    /// Deploys the required dependencies to run a JavaFX application in order to be able to be used by j4rs.
    fn deploy_javafx_dependencies(&self) -> errors::Result<()>;
}

impl JavaFxSupport for Jvm {
    /// Triggers the start of a JavaFX application.
    /// When the JavaFX application starts, the `InstanceReceiver` channel will receive an Instance of `javafx.stage.Stage`.
    ///
    /// The UI may start being built using the provided `Stage`
    fn start_javafx_app(&self) -> errors::Result<InstanceReceiver> {
        let fx_callback = self.create_instance(
            "org.astonbitecode.j4rs.api.jfx.FxApplicationStartCallback",
            &[])?;
        let cb = self.init_callback_channel(&fx_callback)?;
        self.invoke(&fx_callback, "setCallbackToApplicationAndLaunch", &[])?;
        Ok(cb)
    }

    /// Deploys the required dependencies to run a JavaFX application in order to be able to be used by j4rs.
    fn deploy_javafx_dependencies(&self) -> errors::Result<()> {
        let target_os_res = env::var("CARGO_CFG_TARGET_OS");
        if target_os_res.is_ok() {
            let target_os = target_os_res.as_ref().map(|x| &**x).unwrap_or("unknown");
            if target_os == "android" {
                return Ok(());
            }

            println!("cargo:warning=Downloading javafx dependencies from Maven...");
            maven("org.openjfx:javafx-base:13.0.2", self);
            maven(&format!("org.openjfx:javafx-base:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-controls:13.0.2", self);
            maven(&format!("org.openjfx:javafx-controls:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-fxml:13.0.2", self);
            maven(&format!("org.openjfx:javafx-fxml:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-graphics:13.0.2", self);
            maven(&format!("org.openjfx:javafx-graphics:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-media:13.0.2", self);
            maven(&format!("org.openjfx:javafx-media:13.0.2:{}", target_os), self);

            Ok(())
        } else {
            Err(J4RsError::GeneralError("deploy_javafx_dependencies can be used only during build time. It should be called by a build script.".to_string()))
        }
    }
}

fn maven(s: &str, jvm: &Jvm) {
    let artifact = MavenArtifact::from(s);
    let _ = jvm.deploy_artifact(&artifact).map_err(|error| {
        println!("cargo:warning=Could not download Maven artifact {}: {:?}", s, error);
    });
}

#[cfg(test)]
mod api_unit_tests {
    use super::*;
    use crate::JvmBuilder;

    #[test]
    #[should_panic]
    fn test_deploy_javafx_dependencies() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();
        jvm.deploy_javafx_dependencies().unwrap();
    }
}