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

use std::convert::Infallible;
use std::env::VarError;
use std::error::Error;
use std::ffi::NulError;
use std::io;
use std::sync::mpsc::RecvError;
use std::sync::{PoisonError, TryLockError};
use std::{fmt, result};

use fs_extra;
use serde_json;

use futures::channel::oneshot::Canceled;

pub type Result<T> = result::Result<T, J4RsError>;

pub(crate) fn opt_to_res<T>(opt: Option<T>) -> Result<T> {
    opt.ok_or(J4RsError::RustError("Option was found None while converting to result".to_string()))
}

#[allow(unused)]
pub(crate) fn res_to_opt<T>(res: Result<T>) -> Option<T> {
    if res.is_err() {
        None
    } else {
        Some(res.unwrap())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum J4RsError {
    GeneralError(String),
    JavaError(String),
    JniError(String),
    RustError(String),
    ParseError(String),
    Timeout,
}

impl fmt::Display for J4RsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            J4RsError::GeneralError(message) => write!(f, "{}", message),
            J4RsError::JavaError(message) => write!(f, "{}", message),
            J4RsError::JniError(message) => write!(f, "{}", message),
            J4RsError::RustError(message) => write!(f, "{}", message),
            J4RsError::ParseError(message) => write!(f, "{}", message),
            &J4RsError::Timeout => write!(f, "Timeout"),
        }
    }
}

impl Error for J4RsError {
    fn description(&self) -> &str {
        match *self {
            J4RsError::GeneralError(_) => "A general error occured",
            J4RsError::JavaError(_) => "An error coming from Java occured",
            J4RsError::JniError(_) => "A JNI error occured",
            J4RsError::RustError(_) => "An error coming from Rust occured",
            J4RsError::ParseError(_) => "A parsing error occured",
            J4RsError::Timeout => "Timeout",
        }
    }
}

impl From<NulError> for J4RsError {
    fn from(err: NulError) -> J4RsError {
        J4RsError::JniError(format!("{:?}", err))
    }
}

impl From<io::Error> for J4RsError {
    fn from(err: io::Error) -> J4RsError {
        J4RsError::GeneralError(format!("{:?}", err))
    }
}

impl From<serde_json::Error> for J4RsError {
    fn from(err: serde_json::Error) -> J4RsError {
        J4RsError::ParseError(format!("{:?}", err))
    }
}

impl From<fs_extra::error::Error> for J4RsError {
    fn from(err: fs_extra::error::Error) -> J4RsError {
        J4RsError::GeneralError(format!("{:?}", err))
    }
}

impl<T> From<TryLockError<T>> for J4RsError {
    fn from(err: TryLockError<T>) -> J4RsError {
        J4RsError::GeneralError(format!("{:?}", err))
    }
}

impl<T> From<PoisonError<T>> for J4RsError {
    fn from(err: PoisonError<T>) -> J4RsError {
        J4RsError::GeneralError(format!("{:?}", err))
    }
}

impl From<Infallible> for J4RsError {
    fn from(err: Infallible) -> J4RsError {
        J4RsError::RustError(format!("{:?}", err))
    }
}

impl From<RecvError> for J4RsError {
    fn from(err: RecvError) -> J4RsError {
        J4RsError::RustError(format!("{:?}", err))
    }
}

impl From<VarError> for J4RsError {
    fn from(err: VarError) -> J4RsError {
        J4RsError::RustError(format!("{:?}", err))
    }
}

impl From<Canceled> for J4RsError {
    fn from(err: Canceled) -> J4RsError {
        J4RsError::RustError(format!("{:?}", err))
    }
}
