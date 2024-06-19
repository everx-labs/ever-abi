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

#[derive(Debug, thiserror::Error)]
pub enum AbiError {
    #[error("Invalid data: {}", .msg)]
    InvalidData { msg: String },

    #[error("{} is not supported in ABI v{}", .subject, .version)]
    NotSupported {
        subject: String,
        version: AbiVersion,
    },

    #[error("Invalid name: {}", .name)]
    InvalidName { name: String },

    #[error("Invalid function id: {:X}", .id)]
    InvalidFunctionId { id: u32 },

    #[error("Deserialization error {}: {}", .msg, .cursor)]
    DeserializationError {
        msg: &'static str,
        cursor: ever_block::SliceData,
    },

    #[error("Not implemented")]
    NotImplemented,

    #[error(
        "Wrong parameters count. Expected: {}, provided: {}",
        .expected, .provided
    )]
    WrongParametersCount { expected: usize, provided: usize },

    #[error("Token types do not match expected function parameter types")]
    WrongParameterType,

    #[error(
        "Wrong data format in `{}` parameter:\n{}\n{} expected",
        .name, .val, .expected
    )]
    WrongDataFormat {
        val: serde_json::Value,
        name: String,
        expected: String,
    },

    #[error(
        "Invalid parameter `{}` length, expected {}:\n{}",
        .name, .expected, .val
    )]
    InvalidParameterLength {
        name: String,
        val: serde_json::Value,
        expected: String,
    },

    #[error("Invalid parameter `{}` value:\n{}\n{}", .name, .val, .err)]
    InvalidParameterValue {
        name: String,
        val: serde_json::Value,
        err: String,
    },

    #[error("Incomplete deserialization error")]
    IncompleteDeserializationError,

    #[error("Invalid input data: {}", .msg)]
    InvalidInputData { msg: String },

    #[error("Invalid version: {}", .0)]
    InvalidVersion(String),

    #[error("Wrong function ID: {:x}", .id)]
    WrongId { id: u32 },

    #[error("Serde json error: {}", .err)]
    SerdeError { err: serde_json::Error },

    #[error("Tuple description should contain non empty `components` field")]
    EmptyComponents,

    #[error(
        "Type description contains non empty `components` field but it is not a tuple"
    )]
    UnusedComponents,

    #[error(
        "Message destination address is required to encode signed external inbound message body since ABI version 2.3"
    )]
    AddressRequired,

    #[error("Wrong data layout")]
    WrongDataLayout
}
