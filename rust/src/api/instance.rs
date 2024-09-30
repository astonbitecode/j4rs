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

use crate::logger::debug;
use crate::{cache, errors, jni_utils, InvocationArg, Jvm};
use jni_sys::jobject;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::any::Any;
use std::convert::TryFrom;
use std::sync::mpsc::{Receiver, Sender};

/// A Java instance
/// Instances contain global Java references and can be sent to other threads
#[derive(Serialize)]
pub struct Instance {
    /// The name of the class of this instance
    pub(crate) class_name: String,
    /// The JNI jobject that manipulates this instance.
    ///
    /// This object is an instance of `org/astonbitecode/j4rs/api/Instance`
    #[serde(skip)]
    pub(crate) jinstance: jobject,
    #[serde(skip)]
    pub(crate) skip_deleting_jobject: bool,
}

impl Instance {
    /// Creates a new Instance, leaving the passed jobject as is.
    /// In most cases, the jobject is already transformed to a global reference.
    pub(crate) fn new(obj: jobject, classname: &str) -> errors::Result<Instance> {
        Ok(Instance {
            jinstance: obj,
            class_name: classname.to_string(),
            skip_deleting_jobject: false,
        })
    }

    /// Returns the class name of this instance
    pub fn class_name(&self) -> &str {
        self.class_name.as_ref()
    }

    /// Consumes the Instance and returns its jobject
    pub fn java_object(mut self) -> jobject {
        self.skip_deleting_jobject = true;
        self.jinstance
    }

    #[deprecated(
        since = "0.12.0",
        note = "Please use Instance::from_jobject or Instance::from_jobject_with_global_ref instead"
    )]
    pub fn from(obj: jobject) -> errors::Result<Instance> {
        let _jvm = cache::get_thread_local_env().map_err(|_| Jvm::attach_thread());

        let global =
            jni_utils::create_global_ref_from_local_ref(obj, cache::get_thread_local_env()?)?;
        Ok(Instance {
            jinstance: global,
            class_name: cache::UNKNOWN_FOR_RUST.to_string(),
            skip_deleting_jobject: false,
        })
    }

    pub fn from_jobject(obj: jobject) -> errors::Result<Instance> {
        let _jvm = cache::get_thread_local_env().map_err(|_| Jvm::attach_thread());

        Ok(Instance {
            jinstance: obj,
            class_name: cache::UNKNOWN_FOR_RUST.to_string(),
            skip_deleting_jobject: false,
        })
    }

    pub fn from_jobject_with_global_ref(obj: jobject) -> errors::Result<Instance> {
        let _jvm = cache::get_thread_local_env().map_err(|_| Jvm::attach_thread());

        let global =
            jni_utils::create_global_ref_from_local_ref(obj, cache::get_thread_local_env()?)?;
        Ok(Instance {
            jinstance: global,
            class_name: cache::UNKNOWN_FOR_RUST.to_string(),
            skip_deleting_jobject: false,
        })
    }

    /// Creates a weak reference of this Instance.
    fn _weak_ref(&self) -> errors::Result<Instance> {
        Ok(Instance {
            class_name: self.class_name.clone(),
            jinstance: jni_utils::_create_weak_global_ref_from_global_ref(
                self.jinstance.clone(),
                cache::get_thread_local_env()?,
            )?,
            skip_deleting_jobject: false,
        })
    }
}

impl TryFrom<InvocationArg> for Instance {
    type Error = errors::J4RsError;
    fn try_from(invocation_arg: InvocationArg) -> errors::Result<Instance> {
        let obj = invocation_arg.as_java_ptr_with_local_ref(cache::get_thread_local_env()?)?;
        Instance::new(obj, invocation_arg.class_name())
    }
}

impl TryFrom<jobject> for Instance {
    type Error = errors::J4RsError;
    fn try_from(obj: jobject) -> errors::Result<Instance> {
        Instance::from_jobject_with_global_ref(obj)
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        debug(&format!("Dropping an instance of {}", self.class_name));
        if !self.skip_deleting_jobject {
            if let Some(j_env) = cache::get_thread_local_env_opt() {
                jni_utils::delete_java_ref(j_env, self.jinstance);
            }
        }
    }
}

/// Instances contain global Java references and can be sent to other threads
unsafe impl Send for Instance {}

/// A receiver for Java Instances.
///
/// It keeps a channel Receiver to get callback Instances from the Java world
/// and the address of a `Box<Sender<Instance>>` Box in the heap. This Box is used by Java to communicate
/// asynchronously Instances to Rust.
///
/// On Drop, the InstanceReceiver removes the Box from the heap.
pub struct InstanceReceiver {
    pub(crate) rx: Box<Receiver<Instance>>,
    tx_address: u64,
}

impl InstanceReceiver {
    pub(crate) fn new(rx: Receiver<Instance>, tx_address: u64) -> InstanceReceiver {
        InstanceReceiver {
            rx: Box::new(rx),
            tx_address,
        }
    }

    pub fn rx(&self) -> &Receiver<Instance> {
        &self.rx
    }
}

impl Drop for InstanceReceiver {
    fn drop(&mut self) {
        if self.tx_address > 0 {
            debug("Dropping an InstanceReceiver");
            let p = self.tx_address as *mut Sender<Instance>;
            unsafe {
                let tx = Box::from_raw(p);
                drop(tx);
            }
        }
    }
}

/// Allows chained Jvm calls to created Instances
pub struct ChainableInstance<'a> {
    instance: Instance,
    jvm: &'a Jvm,
}

impl<'a> ChainableInstance<'a> {
    pub(crate) fn new(instance: Instance, jvm: &'a Jvm) -> ChainableInstance {
        ChainableInstance { instance, jvm }
    }

    pub(crate) fn new_with_instance_ref(
        instance: &Instance,
        jvm: &'a Jvm,
    ) -> errors::Result<ChainableInstance<'a>> {
        let cloned = jvm.clone_instance(&instance)?;
        Ok(ChainableInstance {
            instance: cloned,
            jvm,
        })
    }

    pub fn collect(self) -> Instance {
        self.instance
    }

    /// Invokes the method `method_name` of a this `Instance`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn invoke(
        &self,
        method_name: &str,
        inv_args: &[InvocationArg],
    ) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.invoke(&self.instance, method_name, inv_args)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Creates a clone of the Instance
    pub fn clone_instance(&self) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.clone_instance(&self.instance)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Invokes the static method `method_name` of the class `class_name`, passing an array of `InvocationArg`s. It returns an `Instance` as the result of the invocation.
    pub fn cast(&self, to_class: &str) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.cast(&self.instance, to_class)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Retrieves the field `field_name` of the `Instance`.
    pub fn field(&self, field_name: &str) -> errors::Result<ChainableInstance> {
        let instance = self.jvm.field(&self.instance, field_name)?;
        Ok(ChainableInstance::new(instance, self.jvm))
    }

    /// Returns the Rust representation of the provided instance
    pub fn to_rust<T: Any>(self) -> errors::Result<T>
    where
        T: DeserializeOwned,
    {
        self.jvm.to_rust(self.instance)
    }

    /// Returns the Rust representation of the provided instance, boxed
    pub fn to_rust_boxed<T: Any>(self) -> errors::Result<Box<T>>
    where
        T: DeserializeOwned,
    {
        self.jvm.to_rust_boxed(self.instance)
    }
}

#[cfg(test)]
mod instance_unit_tests {
    use crate::*;
    use crate::lib_unit_tests::create_tests_jvm;

    #[test]
    fn is_null() -> errors::Result<()> {
        let jvm = create_tests_jvm()?;
        let test_instance = jvm
            .create_instance("org.astonbitecode.j4rs.tests.MyTest", InvocationArg::empty())
            ?;
        let maybe_null = jvm.invoke(&test_instance, "getNullInteger", InvocationArg::empty())?;
        let is_null =
            jvm.invoke_static(
                "java.util.Objects", 
                "isNull", 
                &[InvocationArg::try_from(maybe_null)?])?;
        let is_null: bool = jvm.to_rust(is_null)?;
        assert_eq!(is_null, true);
        Ok(())
    }

    #[test]
    fn try_from_jobject() -> errors::Result<()> {
        let c = std::ptr::null_mut();
        let instance = Instance::try_from(c)?;
        assert!(instance.java_object() == std::ptr::null_mut());
        Ok(())
    }
}