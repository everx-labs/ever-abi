/*
* Copyright (C) 2019-2023 EverX. All Rights Reserved.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific EVERX DEV software governing permissions and
* limitations under the License.
*/

use crate::contract::AbiVersion;

#[derive(Debug, failure::Fail)]
pub enum AbiError {
    #[fail(display = "Invalid data: {}", msg)]
    InvalidData { msg: String },

    #[fail(display = "{} is not supported in ABI v{}", subject, version)]
    NotSupported {
        subject: String,
        version: AbiVersion,
    },

    #[fail(display = "Invalid name: {}", name)]
    InvalidName { name: String },

    #[fail(display = "Invalid function id: {:X}", id)]
    InvalidFunctionId { id: u32 },

    #[fail(display = "Deserialization error {}: {}", msg, cursor)]
    DeserializationError {
        msg: &'static str,
        cursor: ever_types::SliceData,
    },

    #[fail(display = "Not implemented")]
    NotImplemented,

    #[fail(
        display = "Wrong parameters count. Expected: {}, provided: {}",
        expected, provided
    )]
    WrongParametersCount { expected: usize, provided: usize },

    #[fail(display = "Token types do not match expected function parameter types")]
    WrongParameterType,

    #[fail(
        display = "Wrong data format in `{}` parameter:\n{}\n{} expected",
        name, val, expected
    )]
    WrongDataFormat {
        val: serde_json::Value,
        name: String,
        expected: String,
    },

    #[fail(
        display = "Invalid parameter `{}` length, expected {}:\n{}",
        name, expected, val
    )]
    InvalidParameterLength {
        name: String,
        val: serde_json::Value,
        expected: String,
    },

    #[fail(display = "Invalid parameter `{}` value:\n{}\n{}", name, val, err)]
    InvalidParameterValue {
        name: String,
        val: serde_json::Value,
        err: String,
    },

    #[fail(display = "Incomplete deserialization error")]
    IncompleteDeserializationError,

    #[fail(display = "Invalid input data: {}", msg)]
    InvalidInputData { msg: String },

    #[fail(display = "Invalid version: {}", 0)]
    InvalidVersion(String),

    #[fail(display = "Wrong function ID: {:x}", id)]
    WrongId { id: u32 },

    #[fail(display = "Serde json error: {}", err)]
    SerdeError { err: serde_json::Error },

    #[fail(display = "Tuple description should contain non empty `components` field")]
    EmptyComponents,

    #[fail(
        display = "Type description contains non empty `components` field but it is not a tuple"
    )]
    UnusedComponents,

    #[fail(
        display = "Message destination address is required to encode signed external inbound message body since ABI version 2.3"
    )]
    AddressRequired,

    #[fail(display = "Wrong data layout")]
    WrongDataLayout
}
