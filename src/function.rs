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

//! Contract function call builder.

use crate::{error::AbiError, param::Param, token::{Token, TokenValue}};
 
use std::collections::HashMap;
use sha2::{Digest, Sha256};
use contract::SerdeFunction;
use ed25519::signature::Signer;
use ed25519_dalek::{Keypair, SIGNATURE_LENGTH};
use ton_block::Serializable;
use ton_types::{BuilderData, Cell, error, fail, IBitstring, Result, SliceData};

/// Contract function specification.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// ABI version
    pub abi_version: u8,
    /// Function name.
    pub name: String,
    /// Function header parameters.
    pub header: Vec<Param>,
    /// Function input.
    pub inputs: Vec<Param>,
    /// Function output.
    pub outputs: Vec<Param>,
    /// Function ID for inbound messages
    pub input_id: u32,
    /// Function ID for outbound messages
    pub output_id: u32,
}

impl Function {
    /// Creates `Function` struct from parsed JSON struct `SerdeFunction`
    pub(crate) fn from_serde(abi_version: u8, serde_function: SerdeFunction, header: Vec<Param>) -> Self {
        let mut function = Function {
            abi_version,
            name: serde_function.name,
            header,
            inputs: serde_function.inputs,
            outputs: serde_function.outputs,
            input_id: 0,
            output_id: 0
        };
        if let Some(id) = serde_function.id {
            function.input_id = id;
            function.output_id = id
        } else {
            let id = function.get_function_id();
            function.input_id = id & 0x7FFFFFFF;
            function.output_id = id | 0x80000000;
        };
        function
    }

    /// Returns all header params of given function.
    pub fn header_params(&self) -> &Vec<Param> {
        &self.header
    }

    /// Returns all input params of given function.
    pub fn input_params(&self) -> &Vec<Param> {
        &self.inputs
    }

    /// Returns all output params of given function.
    pub fn output_params(&self) -> &Vec<Param> {
        &self.outputs
    }

    /// Returns true if function has input parameters, false in not
    pub fn has_input(&self) -> bool {
        self.inputs.len() != 0
    }

    /// Returns true if function has output parameters, false in not
    pub fn has_output(&self) -> bool {
        self.outputs.len() != 0
    }

    /// Retruns ABI function signature
    pub fn get_function_signature(&self) -> String {
        let mut input_types = vec![];
        if self.abi_version == 1 {
            input_types.append(&mut self.header.iter()
                .map(|param| param.kind.type_signature())
                .collect::<Vec<String>>())
        }

        input_types.append(&mut self.inputs.iter()
            .map(|param| param.kind.type_signature())
            .collect::<Vec<String>>());
        
        let input_types = input_types.join(",");

        let output_types = self.outputs.iter()
            .map(|param| param.kind.type_signature())
            .collect::<Vec<String>>()
            .join(",");

        format!("{}({})({})v{}", self.name, input_types, output_types, self.abi_version)
    }

    pub fn calc_function_id(signature: &str) -> u32 {
        // Sha256 hash of signature
        let mut hasher = Sha256::new();

        hasher.input(&signature.as_bytes());

        let function_hash = hasher.result();

        let mut bytes: [u8; 4] = [0; 4];
        bytes.copy_from_slice(&function_hash[..4]);
        //println!("{}: {:X}", signature, u32::from_be_bytes(bytes));

        u32::from_be_bytes(bytes)
    }

    /// Computes function ID for contract function
    pub fn get_function_id(&self) -> u32 {
        let signature = self.get_function_signature();

        Self::calc_function_id(&signature)
    }

       /// Returns ID for call message
    pub fn get_input_id(&self) -> u32 {
        self.input_id
    }

    /// Returns ID for response message
    pub fn get_output_id(&self) -> u32 {
        self.output_id
    }

    /// Parses the ABI function output to list of tokens.
    pub fn decode_output(&self, mut data: SliceData, _internal: bool) -> Result<Vec<Token>> {
        let id = data.get_next_u32()?;
        if id != self.get_output_id() { Err(AbiError::WrongId { id } )? }
        TokenValue::decode_params(self.output_params(), data, self.abi_version)
    }

    /// Parses the ABI function call to list of tokens.
    pub fn decode_input(&self, data: SliceData, internal: bool) -> Result<Vec<Token>> {
        let (_, id, cursor) = Self::decode_header(self.abi_version, data, &self.header, internal)?;

        if id != self.get_input_id() { Err(AbiError::WrongId { id } )? }

        TokenValue::decode_params(self.input_params(), cursor, self.abi_version)
    }

    /// Decodes function id from contract answer
    pub fn decode_input_id(
        abi_version: u8,
        cursor: SliceData,
        header: &Vec<Param>,
        internal: bool
    ) -> Result<u32> {
        let (_, id, _) = Self::decode_header(abi_version, cursor, header, internal)?;
        Ok(id)
    }

    /// Decodes function id from contract answer
    pub fn decode_output_id(mut data: SliceData) -> Result<u32> {
        Ok(data.get_next_u32()?)
    }

    /// Encodes provided function parameters into `BuilderData` containing ABI contract call
    pub fn encode_input(
        &self,
        header: &HashMap<String, TokenValue>,
        input: &[Token],
        internal: bool,
        pair: Option<&Keypair>
    ) -> Result<BuilderData> {
        let (mut builder, hash) = self.create_unsigned_call(header, input, internal, pair.is_some())?;

        if !internal {
            builder = match pair {
                Some(pair) => {
                    let signature = pair.sign(&hash).to_bytes().to_vec();
                    Self::fill_sign(
                        self.abi_version,
                        Some(&signature),
                        Some(&pair.public.to_bytes()),
                        builder)?
                },
                None => Self::fill_sign(self.abi_version, None, None, builder)?
            }
        }

        Ok(builder)
    }

    /// Encodes provided function return values into `BuilderData`
    pub fn encode_internal_output(
        &self,
        answer_id: u32,
        input: &[Token]
    ) -> Result<BuilderData> {
        let mut cells = vec![];
        cells.push(answer_id.write_to_new_cell()?);
        let values : Vec<TokenValue> = input.iter().map(|x| x.value.clone()).collect();
        let builder = TokenValue::pack_values_into_chain(&values, cells, self.abi_version)?.0;
        Ok(builder)
    }

    /// Encodes function header with provided header parameters
    fn encode_header(
        &self,
        header_tokens: &HashMap<String, TokenValue>,
        internal: bool
    ) -> Result<Vec<BuilderData>> {
        let mut vec = vec![];
        if !internal {
            for param in &self.header {
                if let Some(token) = header_tokens.get(&param.name) {
                    if !token.type_check(&param.kind) {
                        return Err(AbiError::WrongParameterType.into());
                    }
                    vec.push(token.pack_into_chain(self.abi_version)?.0);
                } else {
                    vec.push(TokenValue::get_default_value_for_header(&param.kind)?.pack_into_chain(self.abi_version)?.0);
                }
            }
        }
        if self.abi_version == 1 {
            vec.insert(0, self.get_input_id().write_to_new_cell()?);
        } else {
            vec.push(self.get_input_id().write_to_new_cell()?);
        }
        Ok(vec)
    }

    /// Encodes function header with provided header parameters
    pub fn decode_header(
        abi_version: u8,
        mut cursor: SliceData,
        header: &Vec<Param>,
        internal: bool
    ) -> Result<(Vec<Token>, u32, SliceData)> {
        let mut tokens = vec![];
        let mut id = 0;
        if abi_version == 1 {
            id = cursor.get_next_u32()?;
        }
        if !internal {
            // skip signature
            if abi_version == 1 {
                cursor.checked_drain_reference()?;
            } else {
                if cursor.get_next_bit()? {
                    cursor.get_next_bytes(ed25519_dalek::SIGNATURE_LENGTH)?;
                }
            }

            for param in header {
                let (token_value, new_cursor) = TokenValue::read_from(&param.kind, cursor, false, abi_version)?;
    
                cursor = new_cursor;
                tokens.push(Token { name: param.name.clone(), value: token_value });
            }
        }
        if abi_version != 1 {
            id = cursor.get_next_u32()?;
        }
        Ok((tokens, id, cursor))
    }

    /// Encodes provided function parameters into `BuilderData` containing ABI contract call.
    /// `BuilderData` is prepared for signing. Sign should be the added by `add_sign_to_function_call` function
    pub fn create_unsigned_call(
        &self,
        header: &HashMap<String, TokenValue>,
        input: &[Token],
        internal: bool,
        reserve_sign: bool
    ) -> Result<(BuilderData, Vec<u8>)> {
        let params = self.input_params();

        if !Token::types_check(input, params.as_slice()) {
            fail!(AbiError::WrongParameterType);
        }

        // prepare standard message
        let mut cells = self.encode_header(header, internal)?;

        let mut remove_ref = false;
        let mut remove_bits = 0;
        if !internal {
            let mut sign_builder = BuilderData::new();
            if self.abi_version == 1 {
                // reserve reference for sign
                sign_builder.append_reference(BuilderData::new());
                remove_ref = true;
            } else {
                // reserve in-cell data
                if reserve_sign {
                    sign_builder.append_bit_one()?;
                    sign_builder.append_raw(&[0u8; SIGNATURE_LENGTH], SIGNATURE_LENGTH * 8)?;
                    remove_bits = 1 + SIGNATURE_LENGTH * 8;
                } else {
                    sign_builder.append_bit_zero()?;
                    remove_bits = 1;
                }
            }
            cells.insert(0, sign_builder);
        }

        // encoding itself
        let values : Vec<TokenValue> = input.iter().map(|x| x.value.clone()).collect();
        let mut builder = TokenValue::pack_values_into_chain(&values, cells, self.abi_version)?.0;

        if !internal {
            // delete reserved sign before hash
            let mut slice = SliceData::from(builder);
            if remove_ref {
                slice.checked_drain_reference()?;
            }
            if remove_bits != 0 {
                slice.get_next_bits(remove_bits)?;
            }
            builder = BuilderData::from_slice(&slice);
        }

        let hash = Cell::from(&builder).repr_hash().as_slice().to_vec();

        Ok((builder, hash))
    }

    /// Add sign to messsage body returned by `prepare_input_for_sign` function
    pub fn fill_sign(
        abi_version: u8,
        signature: Option<&[u8]>,
        public_key: Option<&[u8]>,
        mut builder: BuilderData
    ) -> Result<BuilderData> {

        if abi_version == 1 {
            // sign in reference
            if builder.references_free() == 0 {
                fail!(AbiError::InvalidInputData { msg: "No free reference for signature".to_owned() } );
            }
            if let Some(signature) = signature {
                let mut signature = signature.to_vec();
                if let Some(public_key) = public_key {
                    signature.extend_from_slice(public_key);
                }
        
                let len = signature.len() * 8;
                builder.prepend_reference(BuilderData::with_raw(signature, len).unwrap());
            } else {
                builder.prepend_reference(BuilderData::new());
            }
        } else {
            // sign in cell body
            let mut sign_builder = BuilderData::new();
            if let Some(signature) = signature {
                let len = signature.len() * 8;
                sign_builder.append_bit_one()?;
                sign_builder.append_raw(&signature, len)?;
            } else {
                sign_builder.append_bit_zero()?;
            }
            builder.prepend_builder(&sign_builder)?;
        }

        Ok(builder)
    }

    /// Add sign to messsage body returned by `prepare_input_for_sign` function
    pub fn add_sign_to_encoded_input(
        abi_version: u8,
        signature: &[u8],
        public_key: Option<&[u8]>,
        function_call: SliceData
    ) -> Result<BuilderData> {
        let builder = BuilderData::from_slice(&function_call);

        Self::fill_sign(abi_version, Some(signature), public_key, builder)
    }

    /// Check if message body is related to this function
    pub fn is_my_input_message(&self, data: SliceData, internal: bool) -> Result<bool> {
        let decoded_id = Self::decode_input_id(self.abi_version, data, &self.header, internal)?;
        Ok(self.get_input_id() == decoded_id)
    }

    /// Check if message body is related to this function
    pub fn is_my_output_message(&self, data: SliceData, _internal: bool) -> Result<bool> {
        let decoded_id = Self::decode_output_id(data)?;
        Ok(self.get_output_id() == decoded_id)
    }
}
