/*
* Copyright 2018-2020 TON DEV SOLUTIONS LTD.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.  You may obtain a copy of the
* License at: https://ton.dev/licenses
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific TON DEV software governing permissions and
* limitations under the License.
*/

use failure::{Context, Fail, Backtrace};
use std::fmt::{Formatter, Result, Display};

#[derive(Debug)]
pub struct AbiError {
    inner: Context<AbiErrorKind>,
}

pub type AbiResult<T> = std::result::Result<T, failure::Error>;

#[derive(Debug, Fail)]
pub enum AbiErrorKind {

    #[fail(display = "Block error: {}", error)]
    BlockError {
        error: ton_block::BlockError
    },

    #[fail(display = "Invalid data: {}", msg)]
    InvalidData {
        msg: String
    },

    #[fail(display = "Invalid name: {}", name)]
    InvalidName {
        name: String
    },

    #[fail(display = "Invalid function id: {}", id)]
    InvalidFunctionId {
        id: u32
    },

    #[fail(display = "Deserialization error {}: {}", msg, cursor)]
    DeserializationError {
        msg: &'static str,
        cursor: ton_types::SliceData
    },

    #[fail(display = "Not implemented")]
    NotImplemented,

    #[fail(display = "Wrong parameters count. Expected: {}, provided: {}", expected, provided)]
    WrongParametersCount {
        expected: usize,
        provided: usize
    },

    #[fail(display = "Wrong parameter type")]
    WrongParameterType,

    #[fail(display = "Wrong data format:\n{}", val)]
    WrongDataFormat {
        val: serde_json::Value
    },

    #[fail(display = "Invalid parameter length:\n{}", val)]
    InvalidParameterLength {
        val: serde_json::Value
    },

    #[fail(display = "Invalid parameter value:\n{}", val)]
    InvalidParameterValue {
        val: serde_json::Value
    },

    #[fail(display = "Incomplete deserialization error: {}", cursor)]
    IncompleteDeserializationError {
        cursor: ton_types::SliceData
    },

    #[fail(display = "Invalid input data: {}", msg)]
    InvalidInputData {
        msg: String
    },

    #[fail(display = "Wrong version: {}", version)]
    WrongVersion {
        version: u8
    },

    #[fail(display = "Wrong function ID: {:x}", id)]
    WrongId {
        id: u32
    },

    #[fail(display = "IO error: {}", err)]
    Io { 
        err: std::io::Error
    },

    #[fail(display = "Serde json error: {}", err)]
    SerdeError {
        err: serde_json::Error
    },

    #[fail(display = "VM exception: {}", ex)]
    TvmException {
        ex: ton_vm::types::Exception,
    },

    #[fail(display = "VM exception, code: {}", code)]
    TvmExceptionCode {
        code: ton_types::types::ExceptionCode,
    },

    #[fail(display = "Try from int error: {}", err)]
    TryFromIntError {
        err: std::num::TryFromIntError
    },
}

impl AbiError {
    pub fn kind(&self) -> &AbiErrorKind {
        self.inner.get_context()
    }
}

impl Fail for AbiError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for AbiError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Display::fmt(&self.inner, f)
    }
}

impl From<AbiErrorKind> for AbiError {
    fn from(kind: AbiErrorKind) -> AbiError {
        AbiError { inner: Context::new(kind) }
    }
}

impl From<ton_types::types::ExceptionCode> for AbiError {
    fn from(code: ton_types::types::ExceptionCode) -> AbiError {
        AbiError::from(AbiErrorKind::TvmExceptionCode { code })
    }
}

impl From<ton_vm::types::Exception> for AbiError {
    fn from(ex: ton_vm::types::Exception) -> AbiError {
        AbiError::from(AbiErrorKind::TvmException { ex })
    }
}

impl From<std::num::TryFromIntError> for AbiError {
    fn from(err: std::num::TryFromIntError) -> AbiError {
        AbiError::from(AbiErrorKind::TryFromIntError { err })
    }
}

impl From<std::io::Error> for AbiError {
    fn from(err: std::io::Error) -> AbiError {
        AbiError::from(AbiErrorKind::Io { err })
    }
}

impl From<serde_json::Error> for AbiError {
    fn from(err: serde_json::Error) -> AbiError {
        AbiError::from(AbiErrorKind::SerdeError { err })
    }
}