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

use crate::{
    error::AbiError, contract::Contract, token::{Detokenizer, Tokenizer, TokenValue}
};

use ed25519_dalek::Keypair;
use serde_json::Value;
use std::collections::HashMap;
use ton_types::{Result, BuilderData, SliceData};

/// Encodes `parameters` for given `function` of contract described by `abi` into `BuilderData`
/// which can be used as message body for calling contract
pub fn encode_function_call(
    abi: String,
    function: String,
    header: Option<String>,
    parameters: String,
    internal: bool,
    pair: Option<&Keypair>,
) -> Result<BuilderData> {
    let contract = Contract::load(abi.as_bytes())?;

    let function = contract.function(&function)?;

    let mut header_tokens = if let Some(header) = header {
        let v: Value = serde_json::from_str(&header).map_err(|err| AbiError::SerdeError { err } )?;
        Tokenizer::tokenize_optional_params(function.header_params(), &v, &HashMap::new())?
    } else {
        HashMap::new()
    };
    // add public key into header
    if pair.is_some() && header_tokens.get("pubkey").is_none() {
        header_tokens.insert("pubkey".to_owned(), TokenValue::PublicKey(pair.map(|pair| pair.public)));
    }

    let v: Value = serde_json::from_str(&parameters).map_err(|err| AbiError::SerdeError { err } )?;
    let input_tokens = Tokenizer::tokenize_all_params(function.input_params(), &v)?;

    function.encode_input(&header_tokens, &input_tokens, internal, pair)
}

/// Encodes `parameters` for given `function` of contract described by `abi` into `BuilderData`
/// which can be used as message body for calling contract. Message body is prepared for
/// signing. Sign should be the added by `add_sign_to_function_call` function
pub fn prepare_function_call_for_sign(
    abi: String,
    function: String,
    header: Option<String>,
    parameters: String,
) -> Result<(BuilderData, Vec<u8>)> {
    let contract = Contract::load(abi.as_bytes())?;

    let function = contract.function(&function)?;

    let header_tokens = if let Some(header) = header {
        let v: Value = serde_json::from_str(&header).map_err(|err| AbiError::SerdeError { err } )?;
        Tokenizer::tokenize_optional_params(function.header_params(), &v, &HashMap::new())?
    } else {
        HashMap::new()
    };

    let v: Value = serde_json::from_str(&parameters).map_err(|err| AbiError::SerdeError { err } )?;
    let input_tokens = Tokenizer::tokenize_all_params(function.input_params(), &v)?;

    function.create_unsigned_call(&header_tokens, &input_tokens, false, true)
}

/// Add sign to messsage body returned by `prepare_function_call_for_sign` function
pub fn add_sign_to_function_call(
    abi: String,
    signature: &[u8],
    public_key: Option<&[u8]>,
    function_call: SliceData
) -> Result<BuilderData> {
    let contract = Contract::load(abi.as_bytes())?;
    contract.add_sign_to_encoded_input(signature, public_key, function_call)
}

/// Decodes output parameters returned by contract function call
pub fn decode_function_response(
    abi: String,
    function: String,
    response: SliceData,
    internal: bool,
) -> Result<String> {
    let contract = Contract::load(abi.as_bytes())?;

    let function = contract.function(&function)?;

    let tokens = function.decode_output(response, internal)?;

    Detokenizer::detokenize(&tokens)
}

pub struct DecodedMessage {
    pub function_name: String,
    pub params: String
}

/// Decodes output parameters returned by some function call. Returns parametes and function name
pub fn decode_unknown_function_response(
    abi: String,
    response: SliceData,
    internal: bool,
) -> Result<DecodedMessage> {
    let contract = Contract::load(abi.as_bytes())?;

    let result = contract.decode_output(response, internal)?;

    let output = Detokenizer::detokenize(&result.tokens)?;

    Ok(DecodedMessage {
        function_name: result.function_name,
        params: output
    })
}

/// Decodes output parameters returned by some function call. Returns parametes and function name
pub fn decode_unknown_function_call(
    abi: String,
    response: SliceData,
    internal: bool,
) -> Result<DecodedMessage> {
    let contract = Contract::load(abi.as_bytes())?;

    let result = contract.decode_input(response, internal)?;

    let input = Detokenizer::detokenize(&result.tokens)?;

    Ok(DecodedMessage {
        function_name: result.function_name,
        params: input
    })
}

/// Changes initial values for public contract variables
pub fn update_contract_data(abi: &str, parameters: &str, data: SliceData) -> Result<SliceData> {
    let contract = Contract::load(abi.as_bytes())?;

    let data_json: serde_json::Value = serde_json::from_str(parameters)?;

    let params: Vec<_> = contract
        .data()
        .values()
        .map(|item| item.value.clone())
        .collect();

    let tokens = Tokenizer::tokenize_all_params(&params[..], &data_json)?;

    contract.update_data(data, &tokens)
}

/// Decode initial values of public contract variables
pub fn decode_contract_data(abi: &str, data: SliceData) -> Result<String> {
    let contract = Contract::load(abi.as_bytes())?;

    Detokenizer::detokenize(&contract.decode_data(data)?)
}

/// Decode account storage fields
pub fn decode_storage_fields(abi: &str, data: SliceData) -> Result<String> {
    let contract = Contract::load(abi.as_bytes())?;

    let decoded = contract.decode_storage_fields(data)?;

    Detokenizer::detokenize(&decoded)
}

#[cfg(test)]
#[path = "tests/v1/full_stack_tests.rs"]
mod tests_v1;

#[cfg(test)]
#[path = "tests/v2/full_stack_tests.rs"]
mod tests_v2;
