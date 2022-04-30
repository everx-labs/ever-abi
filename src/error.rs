/*
* Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific TON DEV software governing permissions and
* limitations under the License.
*/

#[derive(Debug, failure::Fail)]
pub enum AbiError {

    #[fail(display = "Invalid data: {}", msg)]
    InvalidData {
        msg: String
    },

    #[fail(display = "Invalid name: {}", name)]
    InvalidName {
        name: String
    },

    #[fail(display = "Invalid function id: {:X}", id)]
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

    #[fail(
        display = "Incorrect parameters provided for {}. Expected ({}): {:?}, provided ({}): {:?}",
        for_what,
        expected_count,
        expected,
        provided_count,
        provided,
    )]
    IncorrectParametersProvided {
        for_what: String,
        expected_count: usize,
        expected: Vec<String>,
        provided_count: usize,
        provided: Vec<String>,
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

    #[fail(display = "Incomplete deserialization error")]
    IncompleteDeserializationError,

    #[fail(display = "Invalid input data: {}", msg)]
    InvalidInputData {
        msg: String
    },

    #[fail(display = "Invalid version: {}", 0)]
    InvalidVersion(String),

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

    #[fail(display = "Try from int error: {}", err)]
    TryFromIntError {
        err: std::num::TryFromIntError
    },

    #[fail(display = "Tuple description should contain non empty `components` field")]
    EmptyComponents,

    #[fail(display = "Type description contains non empty `components` field but it is not a tuple")]
    UnusedComponents,
}

