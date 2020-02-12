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

use {Function, Param, Token, TokenValue};
use contract::SerdeEvent;
use ton_types::SliceData;
use crate::error::*;

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

    /// Decodes provided params from SliceData
    fn decode_params(&self, params: Vec<Param>, mut cursor: SliceData) -> AbiResult<Vec<Token>> {
        let mut tokens = vec![];
        let original = cursor.clone();

        let id = cursor.get_next_u32()?;

        if id != self.get_id() { Err(AbiErrorKind::WrongId { id } )? }

        for param in params {
            let (token_value, new_cursor) = TokenValue::read_from(&param.kind, cursor)?;

            cursor = new_cursor;
            tokens.push(Token { name: param.name, value: token_value });
        }

        if cursor.remaining_references() != 0 || cursor.remaining_bits() != 0 {
            bail!(AbiErrorKind::IncompleteDeserializationError { cursor: original } )
        } else {
            Ok(tokens)
        }
    }

    /// Parses the ABI function call to list of tokens.
    pub fn decode_input(&self, data: SliceData) -> AbiResult<Vec<Token>> {
        self.decode_params(self.input_params(), data)
    }

    /// Decodes function id from contract answer
    pub fn decode_id(mut data: SliceData) -> AbiResult<u32> {
        Ok(data.get_next_u32()?)
    }

    /// Check if message body is related to this event
    pub fn is_my_message(&self, data: SliceData, _internal: bool) -> AbiResult<bool> {
        let decoded_id = Self::decode_id(data)?;
        Ok(self.get_id() == decoded_id)
    }
}
