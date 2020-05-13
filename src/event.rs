/*
* Copyright 2018-2020 TON DEV SOLUTIONS LTD.
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

use {Function, Param, Token, TokenValue};
use contract::SerdeEvent;
use ton_types::{Result, SliceData};
use crate::error::AbiError;

/// Contract event specification.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// ABI version
    pub abi_version: u8,
    /// Event name.
    pub name: String,
    /// Event input.
    pub inputs: Vec<Param>,
    /// Event ID
    pub id: u32
}

impl Event {
    /// Creates `Function` struct from parsed JSON struct `SerdeFunction`
    pub(crate) fn from_serde(abi_version: u8, serde_event: SerdeEvent) -> Self {
        let mut event = Event {
            abi_version,
            name: serde_event.name,
            inputs: serde_event.inputs,
            id: 0
        };
        event.id = if let Some(id) = serde_event.id {
            id
        } else {
            event.get_function_id() & 0x7FFFFFFF
        };
        event
    }

    /// Returns all input params of given function.
    pub fn input_params(&self) -> Vec<Param> {
        self.inputs.iter()
            .map(|p| p.clone())
            .collect()
    }

    /// Returns true if function has input parameters, false in not
    pub fn has_input(&self) -> bool {
        self.inputs.len() != 0
    }

    /// Retruns ABI function signature
    pub fn get_function_signature(&self) -> String {
        let input_types = self.inputs.iter()
            .map(|param| param.kind.type_signature())
            .collect::<Vec<String>>()
            .join(",");

        format!("{}({})v{}", self.name, input_types, self.abi_version)
    }

    /// Computes function ID for contract function
    pub fn get_function_id(&self) -> u32 {
        let signature = self.get_function_signature();

        Function::calc_function_id(&signature)
    }

    /// Returns ID for event emitting message
    pub fn get_id(&self) -> u32 {
        self.id
    }

    /// Parses the ABI function call to list of tokens.
    pub fn decode_input(&self, mut data: SliceData) -> Result<Vec<Token>> {
        let id = data.get_next_u32()?;

        if id != self.get_id() { Err(AbiError::WrongId { id } )? }

        TokenValue::decode_params(&self.input_params(), data, self.abi_version)
    }

    /// Decodes function id from contract answer
    pub fn decode_id(mut data: SliceData) -> Result<u32> {
        Ok(data.get_next_u32()?)
    }

    /// Check if message body is related to this event
    pub fn is_my_message(&self, data: SliceData, _internal: bool) -> Result<bool> {
        let decoded_id = Self::decode_id(data)?;
        Ok(self.get_id() == decoded_id)
    }
}
