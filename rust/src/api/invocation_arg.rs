// Copyright 2022 astonbitecode
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

use std::any::Any;
use std::convert::TryFrom;
use std::ptr;

use jni_sys::{jobject, JNIEnv};
use serde::Serialize;
use serde_json;

use crate::api::instance::Instance;
use crate::api::{JavaClass, Jvm, Null};
use crate::{cache, errors, jni_utils, utils};

/// Struct that carries an argument that is used for method invocations in Java.
#[derive(Serialize)]
pub enum InvocationArg {
    /// An arg that is created in the Java world.
    Java {
        instance: Instance,
        class_name: String,
        serialized: bool,
    },
    /// A serialized arg that is created in the Rust world.
    Rust {
        json: String,
        class_name: String,
        serialized: bool,
    },
    /// An non-serialized arg created in the Rust world, that contains a Java instance.
    ///
    /// The instance is a Basic Java type, like Integer, Float, String etc.
    RustBasic {
        instance: Instance,
        class_name: String,
        serialized: bool,
    },
}

impl InvocationArg {
    /// Creates a InvocationArg::Rust.
    /// This is default for the Args that are created from the Rust code.
    pub fn new<T>(arg: &T, class_name: &str) -> InvocationArg
        where
            T: Serialize + Any,
    {
        Self::new_2(
            arg,
            class_name,
            cache::get_thread_local_env().expect("Could not find the jni_env in the local cache. Please make sure that you created a Jvm before using Jvm::new"))
            .expect("Could not create the InvocationArg. Please see the logs/console for more details.")
    }

    pub fn new_2<T>(
        arg: &T,
        class_name: &str,
        jni_env: *mut JNIEnv,
    ) -> errors::Result<InvocationArg>
        where
            T: Serialize + Any,
    {
        let arg_any = arg as &dyn Any;
        if let Some(a) = arg_any.downcast_ref::<String>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(
                    jni_utils::global_jobject_from_str(a, jni_env)?,
                    class_name,
                )?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i8>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(
                    jni_utils::global_jobject_from_i8(a, jni_env)?,
                    class_name,
                )?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i16>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(
                    jni_utils::global_jobject_from_i16(a, jni_env)?,
                    class_name,
                )?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i32>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(
                    jni_utils::global_jobject_from_i32(a, jni_env)?,
                    class_name,
                )?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<i64>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(
                    jni_utils::global_jobject_from_i64(a, jni_env)?,
                    class_name,
                )?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<f32>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(
                    jni_utils::global_jobject_from_f32(a, jni_env)?,
                    class_name,
                )?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else if let Some(a) = arg_any.downcast_ref::<f64>() {
            Ok(InvocationArg::RustBasic {
                instance: Instance::new(
                    jni_utils::global_jobject_from_f64(a, jni_env)?,
                    class_name,
                )?,
                class_name: class_name.to_string(),
                serialized: false,
            })
        } else {
            let json = serde_json::to_string(arg)?;
            Ok(InvocationArg::Rust {
                json: json,
                class_name: class_name.to_string(),
                serialized: true,
            })
        }
    }

    fn make_primitive(&mut self) -> errors::Result<()> {
        match utils::primitive_of(self) {
            Some(primitive_repr) => {
                match self {
                    &mut InvocationArg::Java {
                        instance: _,
                        ref mut class_name,
                        serialized: _,
                    } => *class_name = primitive_repr,
                    &mut InvocationArg::Rust {
                        json: _,
                        ref mut class_name,
                        serialized: _,
                    } => *class_name = primitive_repr,
                    &mut InvocationArg::RustBasic {
                        instance: _,
                        ref mut class_name,
                        serialized: _,
                    } => *class_name = primitive_repr,
                };
                Ok(())
            }
            None => Err(errors::J4RsError::JavaError(format!(
                "Cannot transform to primitive: {}",
                utils::get_class_name(&self)
            ))),
        }
    }

    /// Consumes this InvocationArg and transforms it to an InvocationArg that contains a Java primitive, leveraging Java's autoboxing.
    ///
    /// This action can be done by calling `Jvm::cast` of Instances as well (e.g.: jvm.cast(&instance, "int"))
    /// but calling `into_primitive` is faster, as it does not involve JNI calls.
    pub fn into_primitive(self) -> errors::Result<InvocationArg> {
        let mut ia = self;
        ia.make_primitive()?;
        Ok(ia)
    }

    /// Creates a `jobject` from this InvocationArg.
    pub fn as_java_ptr_with_global_ref(&self, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
        match self {
            _s @ &InvocationArg::Java { .. } => {
                jni_utils::invocation_arg_jobject_from_java(&self, jni_env, true)
            }
            _s @ &InvocationArg::Rust { .. } => {
                jni_utils::invocation_arg_jobject_from_rust_serialized(&self, jni_env, true)
            }
            _s @ &InvocationArg::RustBasic { .. } => {
                jni_utils::invocation_arg_jobject_from_rust_basic(&self, jni_env, true)
            }
        }
    }

    /// Creates a `jobject` from this InvocationArg. The jobject contains a local reference.
    pub fn as_java_ptr_with_local_ref(&self, jni_env: *mut JNIEnv) -> errors::Result<jobject> {
        match self {
            _s @ &InvocationArg::Java { .. } => {
                jni_utils::invocation_arg_jobject_from_java(&self, jni_env, false)
            }
            _s @ &InvocationArg::Rust { .. } => {
                jni_utils::invocation_arg_jobject_from_rust_serialized(&self, jni_env, false)
            }
            _s @ &InvocationArg::RustBasic { .. } => {
                jni_utils::invocation_arg_jobject_from_rust_basic(&self, jni_env, false)
            }
        }
    }

    /// Consumes this invocation arg and returns its Instance
    pub fn instance(self) -> errors::Result<Instance> {
        match self {
            InvocationArg::Java { instance: i, .. } => Ok(i),
            InvocationArg::RustBasic { .. } => Err(errors::J4RsError::RustError(format!(
                "Invalid operation: Cannot get the instance of an InvocationArg::RustBasic"
            ))),
            InvocationArg::Rust { .. } => Err(errors::J4RsError::RustError(format!(
                "Cannot get the instance from an InvocationArg::Rust"
            ))),
        }
    }

    pub fn class_name(&self) -> &str {
        match self {
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
        }
    }

    /// Creates an InvocationArg that contains null
    pub fn create_null(null: Null) -> errors::Result<InvocationArg> {
        let class_name: &str = match null {
            Null::String => JavaClass::String,
            Null::Boolean => JavaClass::Boolean,
            Null::Byte => JavaClass::Byte,
            Null::Character => JavaClass::Character,
            Null::Short => JavaClass::Short,
            Null::Integer => JavaClass::Integer,
            Null::Long => JavaClass::Long,
            Null::Float => JavaClass::Float,
            Null::Double => JavaClass::Double,
            Null::List => JavaClass::List,
            Null::Of(class_name) => JavaClass::Of(class_name),
        }
            .into();
        Ok(InvocationArg::RustBasic {
            instance: Instance::new(ptr::null_mut(), class_name)?,
            class_name: class_name.to_string(),
            serialized: false,
        })
    }
}

impl From<Instance> for InvocationArg {
    fn from(instance: Instance) -> InvocationArg {
        let class_name = instance.class_name.to_owned();

        InvocationArg::Java {
            instance: instance,
            class_name: class_name,
            serialized: false,
        }
    }
}

impl TryFrom<Result<Instance, errors::J4RsError>> for InvocationArg {
    type Error = errors::J4RsError;

    fn try_from(
        instance_res: Result<Instance, errors::J4RsError>,
    ) -> errors::Result<InvocationArg> {
        Ok(InvocationArg::from(instance_res?))
    }
}

impl<'a> TryFrom<Null<'a>> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(null: Null) -> errors::Result<InvocationArg> {
        InvocationArg::create_null(null)
    }
}

impl TryFrom<String> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: String) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg,
            JavaClass::String.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [String]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [String]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl<'a> TryFrom<&'a str> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a str) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg.to_string(),
            JavaClass::String.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [&'a str]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [&'a str]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|&elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<bool> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: bool) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg,
            JavaClass::Boolean.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [bool]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [bool]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i8> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i8) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, JavaClass::Byte.into(), cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [i8]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i8]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<char> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: char) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg,
            JavaClass::Character.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [char]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [char]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i16> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i16) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg,
            JavaClass::Short.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [i16]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i16]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg,
            JavaClass::Integer.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [i32]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i32]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<i64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: i64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, JavaClass::Long.into(), cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a [i64]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [i64]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<f32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: f32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg,
            JavaClass::Float.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [f32]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [f32]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<f64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: f64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            &arg,
            JavaClass::Double.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a [f64]> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(vec: &'a [f64]) -> errors::Result<InvocationArg> {
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| InvocationArg::try_from(elem))
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<()> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: ()) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(&arg, JavaClass::Void.into(), cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a String> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a String) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            arg,
            JavaClass::String.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a bool> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a bool) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            arg,
            JavaClass::Boolean.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a i8> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i8) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, JavaClass::Byte.into(), cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a char> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a char) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            arg,
            JavaClass::Character.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a i16> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i16) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, JavaClass::Short.into(), cache::get_thread_local_env()?)
    }
}

impl<'a, 'b> TryFrom<&'a i32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            arg,
            JavaClass::Integer.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a> TryFrom<&'a i64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a i64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, JavaClass::Long.into(), cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a f32> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a f32) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(arg, JavaClass::Float.into(), cache::get_thread_local_env()?)
    }
}

impl<'a> TryFrom<&'a f64> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: &'a f64) -> errors::Result<InvocationArg> {
        InvocationArg::new_2(
            arg,
            JavaClass::Double.into(),
            cache::get_thread_local_env()?,
        )
    }
}

impl<'a, T: 'static> TryFrom<(&'a [T], &'a str)> for InvocationArg
    where
        T: Serialize,
{
    type Error = errors::J4RsError;
    fn try_from(vec: (&'a [T], &'a str)) -> errors::Result<InvocationArg> {
        let (vec, elements_class_name) = vec;
        let jni_env = cache::get_thread_local_env()?;
        let args: errors::Result<Vec<InvocationArg>> = vec
            .iter()
            .map(|elem| {
                InvocationArg::new_2(elem, JavaClass::Of(elements_class_name).into(), jni_env)
            })
            .collect();
        let res =
            Jvm::do_create_java_list(cache::get_thread_local_env()?, cache::J4RS_ARRAY, &args?);
        Ok(InvocationArg::from(res?))
    }
}

impl TryFrom<Result<InvocationArg, errors::J4RsError>> for InvocationArg {
    type Error = errors::J4RsError;
    fn try_from(arg: Result<InvocationArg, errors::J4RsError>) -> errors::Result<InvocationArg> {
        arg
    }
}

#[cfg(test)]
mod inv_arg_unit_tests {
    use serde::Deserialize;

    use crate::{errors, JvmBuilder, MavenArtifact};

    use super::*;

    include!(concat!(env!("OUT_DIR"), "/j4rs_init.rs"));

    fn create_tests_jvm() -> errors::Result<Jvm> {
        let jvm: Jvm = JvmBuilder::new().build()?;
        jvm.deploy_artifact(&MavenArtifact::from(format!("io.github.astonbitecode:j4rs-testing:{}", j4rs_version()).as_str()))?;
        Ok(jvm)
    }

    #[test]
    fn new_invocation_arg() -> errors::Result<()> {
        let _jvm = create_tests_jvm()?;
        let _ = InvocationArg::new(&"something".to_string(), "somethingelse");

        Ok(())
    }

    #[test]
    fn invocation_arg_try_from_basic_types() -> errors::Result<()> {
        let _jvm = create_tests_jvm()?;
        validate_type(InvocationArg::try_from("str")?, "java.lang.String");
        validate_type(
            InvocationArg::try_from("str".to_string())?,
            "java.lang.String",
        );
        validate_type(InvocationArg::try_from(true)?, "java.lang.Boolean");
        validate_type(InvocationArg::try_from(1_i8)?, "java.lang.Byte");
        validate_type(InvocationArg::try_from('c')?, "java.lang.Character");
        validate_type(InvocationArg::try_from(1_i16)?, "java.lang.Short");
        validate_type(InvocationArg::try_from(1_i64)?, "java.lang.Long");
        validate_type(InvocationArg::try_from(0.1_f32)?, "java.lang.Float");
        validate_type(
            InvocationArg::try_from(0.1_f64)?,
            "java.lang.Double",
        );
        validate_type(InvocationArg::try_from(())?, "void");

        validate_type(
            InvocationArg::try_from(&"str".to_string())?,
            "java.lang.String",
        );
        validate_type(InvocationArg::try_from("str")?, "java.lang.String");
        validate_type(InvocationArg::try_from(&true)?, "java.lang.Boolean");
        validate_type(InvocationArg::try_from(&1_i8)?, "java.lang.Byte");
        validate_type(
            InvocationArg::try_from(&'c')?,
            "java.lang.Character",
        );
        validate_type(InvocationArg::try_from(&1_i16)?, "java.lang.Short");
        validate_type(InvocationArg::try_from(&1_i64)?, "java.lang.Long");
        validate_type(
            InvocationArg::try_from(&0.1_f32)?,
            "java.lang.Float",
        );
        validate_type(
            InvocationArg::try_from(&0.1_f64)?,
            "java.lang.Double",
        );

        Ok(())
    }

    #[test]
    fn invocation_into_primitive() -> errors::Result<()> {
        let _jvm: Jvm = create_tests_jvm()?;
        assert!(InvocationArg::try_from(false)?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from(1_i8)?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from(1_i16)?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from(1_i32)?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from(1_i64)?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from(0.1_f32)?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from(0.1_f64)?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from('c')?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from(())?
            .into_primitive()
            .is_ok());
        assert!(InvocationArg::try_from("string")?
            .into_primitive()
            .is_err());

        Ok(())
    }

    #[test]
    fn invocation_arg_for_custom_types() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;

        let my_bean = MyBean {
            someString: "My String In A Bean".to_string(),
            someInteger: 33,
        };
        let ia = InvocationArg::new(&my_bean, "org.astonbitecode.j4rs.tests.MyBean");

        let test_instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
        let string_instance = jvm.invoke(&test_instance, "getTheString", &[ia]).unwrap();

        let rust_string: String = jvm.to_rust(string_instance).unwrap();

        assert!(&rust_string == "My String In A Bean");

        Ok(())
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[allow(non_snake_case)]
    struct MyBean {
        someString: String,
        someInteger: isize,
    }

    fn validate_type(ia: InvocationArg, class: &str) {
        let b = match ia {
            _s @ InvocationArg::Java { .. } => false,
            InvocationArg::Rust {
                class_name,
                json: _,
                ..
            } => class == class_name,
            InvocationArg::RustBasic {
                instance: _,
                class_name,
                serialized: _,
            } => class == class_name,
        };
        assert!(b);
    }
}
