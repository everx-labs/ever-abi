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

//! ABI param and parsing for it.
use crate::{
    error::AbiError, int::{Int, Uint}, param::Param, param_type::ParamType,
    token::{Token, TokenValue}
};

use serde_json::Value;
use std::{collections::HashMap, io::Cursor, str::FromStr};
use num_bigint::{Sign, BigInt, BigUint};
use num_traits::cast::ToPrimitive;
use ton_block::{Grams, MsgAddress};
use ton_types::{deserialize_tree_of_cells, error, fail, Result, BuilderData};
//use ton_types::cells_serialization::deserialize_tree_of_cells;

/// This struct should be used to parse string values as tokens.
pub struct Tokenizer;

impl Tokenizer {
    /// Tries to parse a JSON value as a token of given type.
    pub fn tokenize_parameter(param: &ParamType, value: &Value) -> Result<TokenValue> {
        match param {
            ParamType::Unknown => fail!(AbiError::NotImplemented),
            ParamType::Uint(size) => Self::tokenize_uint(*size, value),
            ParamType::Int(size) => Self::tokenize_int(*size, value),
            ParamType::Bool => Self::tokenize_bool(value),
            ParamType::Tuple(tuple_params) => Self::tokenize_tuple(tuple_params, value),
            ParamType::Array(param_type) => Self::tokenize_array(&param_type, value),
            ParamType::FixedArray(param_type, size) => Self::tokenize_fixed_array(&param_type, *size, value),
            ParamType::Cell => Self::tokenize_cell(value),
            ParamType::Map(key_type, value_type) => Self::tokenize_hashmap(key_type, value_type, value),
            ParamType::Address => {
                let address = MsgAddress::from_str(
                    &value.as_str()
                        .ok_or_else(|| AbiError::WrongDataFormat { val: value.clone() } )?)
                    .map_err(|_| AbiError::WrongDataFormat { val: value.clone() } )?;
                Ok(TokenValue::Address(address))
            }
            ParamType::Bytes => Self::tokenize_bytes(value, None),
            ParamType::FixedBytes(size) => Self::tokenize_bytes(value, Some(*size)),
            ParamType::Gram => Self::tokenize_gram(value),
            ParamType::Time => Self::tokenize_time(value),
            ParamType::Expire => Self::tokenize_expire(value),
            ParamType::PublicKey => Self::tokenize_public_key(value),
        }
    }

    /// Tries to parse parameters from JSON values to tokens.
    pub fn tokenize_all_params(params: &[Param], values: &Value) -> Result<Vec<Token>> {
        if let Value::Object(map) = values {
            let mut tokens = Vec::new();
            for param in params {
                let value = map
                    .get(&param.name)
                    .ok_or_else(|| error!(AbiError::InvalidInputData { 
                        msg: format!("Parameter `{}` is not specified", param.name) 
                    }))?;
                let token_value = Self::tokenize_parameter(&param.kind, value)?;
                tokens.push(Token { name: param.name.clone(), value: token_value});
            }

            Ok(tokens)
        } else {
            fail!(AbiError::WrongDataFormat { val: values.clone() } )
        }
    }

    /// Tries to parse parameters from JSON values to tokens.
    pub fn tokenize_optional_params(
        params: &[Param],
        values: &Value,
        default_values: &HashMap<String, TokenValue>
    ) -> Result<HashMap<String, TokenValue>> {
        if let Value::Object(map) = values {
            let mut map = map.clone();
            let mut tokens = HashMap::new();
            for param in params {
                if let Some(value) = map.remove(&param.name) {
                    let token_value = Self::tokenize_parameter(&param.kind, &value)?;
                    tokens.insert(param.name.clone(), token_value);
                } else if let Some(value) = default_values.get(&param.name){
                    tokens.insert(param.name.clone(), value.clone());
                }
            }
            if !map.is_empty() {
                let unknown = map.iter().map(|(key, _)| key.as_ref()).collect::<Vec<&str>>().join(", ");
                return Err(AbiError::InvalidInputData { 
                    msg: format!("Contract doesn't have following parameters: {}", unknown)
                }.into());
            }
            Ok(tokens)
        } else {
            fail!(AbiError::WrongDataFormat { val: values.clone() } )
        }
    }

    /// Tries to read tokens array from `Value`
    fn read_array(param: &ParamType, value: &Value) -> Result<Vec<TokenValue>> {
        if let Value::Array(array) = value {
            let mut tokens = Vec::new();
            for value in array {
                tokens.push(Self::tokenize_parameter(param, value)?);
            }
            
            Ok(tokens)
        } else {
            fail!(AbiError::WrongDataFormat { val: value.clone() } )
        }
    }

    /// Tries to parse a value as a vector of tokens of fixed size.
    fn tokenize_fixed_array(
        param: &ParamType,
        size: usize, value: &Value
    ) -> Result<TokenValue> {
        let vec = Self::read_array(param, value)?;
        match vec.len() == size {
            true => Ok(TokenValue::FixedArray(vec)),
            false => fail!(AbiError::InvalidParameterLength { val: value.clone() } ),
        }
    }

    /// Tries to parse a value as a vector of tokens.
    fn tokenize_array(param: &ParamType, value: &Value) -> Result<TokenValue> {
        let vec = Self::read_array(param, value)?;

        Ok(TokenValue::Array(vec))
    }

    /// Tries to parse a value as a bool.
    fn tokenize_bool(value: &Value) -> Result<TokenValue> {
        match value {
            Value::Bool(value) => Ok(TokenValue::Bool(value.to_owned())),
            Value::String(string) => {
                match string.as_str() {
                    "true" => Ok(TokenValue::Bool(true)),
                    "false" => Ok(TokenValue::Bool(false)),
                    _ => fail!(AbiError::InvalidParameterValue { val: value.clone() } ),
                }
            }
            _ => fail!(AbiError::InvalidParameterValue { val: value.clone() } ),
        }
    }

    /// Tries to read integer number from `Value`
    fn read_int(value: &Value) -> Result<BigInt> {
        if let Some(number) = value.as_i64() {
            Ok(BigInt::from(number))
        } else if let Some(string) = value.as_str() {
            let result = if string.starts_with("-0x") {
                BigInt::parse_bytes(&string.as_bytes()[3..], 16)
                .map(|number| -number)
            } else if string.starts_with("0x") {
                BigInt::parse_bytes(&string.as_bytes()[2..], 16)
            } else {
                BigInt::parse_bytes(string.as_bytes(), 10)
            };
            match result {
                Some(number) => Ok(number),
                None => fail!(AbiError::InvalidParameterValue { val: value.clone() } )
            }
        } else {
            fail!(AbiError::WrongDataFormat { val: value.clone() } )
        }
    }

    /// Tries to read integer number from `Value`
    fn read_uint(value: &Value) -> Result<BigUint> {
        if let Some(number) = value.as_u64() {
            Ok(BigUint::from(number))
        } else if let Some(string) = value.as_str() {
            let result = if string.starts_with("0x") {
                BigUint::parse_bytes(&string.as_bytes()[2..], 16)
            } else {
                BigUint::parse_bytes(string.as_bytes(), 10)
            };
            match result {
                Some(number) => Ok(number),
                None => fail!(AbiError::InvalidParameterValue { val: value.clone() } )
            }
        } else {
            fail!(AbiError::WrongDataFormat { val: value.clone() } )
        }
    }

    /// Checks if given number can be fit into given bits count
    fn check_int_size(number: &BigInt, size: usize) -> bool {
        // `BigInt::bits` returns fewest bits necessary to express the number, not including
        // the sign and it works well for all values except `-2^n`. Such values can be encoded 
        // using `n` bits, but `bits` function returns `n` (and plus one bit for sign) so we 
        // have to explicitly check such situation by comparing bits sizes of given number 
        // and increased number
        if number.sign() == Sign::Minus && number.bits() != (number + BigInt::from(1)).bits() {
            number.bits() <= size
        } else {
            number.bits() < size
        }
    }

    /// Checks if given number can be fit into given bits count
    fn check_uint_size(number: &BigUint, size: usize) -> bool {
        number.bits() < size
    }

    /// Tries to parse a value as grams.
    fn tokenize_gram(value: &Value) -> Result<TokenValue> {
        let number = Self::read_uint(value)?;

        if !Self::check_uint_size(&number, 120) {
            fail!(AbiError::InvalidParameterValue { val: value.clone() } )
        } else {
            Ok(TokenValue::Gram(Grams::from(number)))
        }
    }

    /// Tries to parse a value as unsigned integer.
    fn tokenize_uint(size: usize, value: &Value) -> Result<TokenValue> {
        let number = Self::read_uint(value)?;

        if !Self::check_uint_size(&number, size + 1) {
            fail!(AbiError::InvalidParameterValue { val: value.clone() } )
        } else {
            Ok(TokenValue::Uint(Uint{number, size}))
        }
    }

    /// Tries to parse a value as signed integer.
    fn tokenize_int(size: usize, value: &Value) -> Result<TokenValue> {
        let number = Self::read_int(value)?;

        if !Self::check_int_size(&number, size) {
            fail!(AbiError::InvalidParameterValue { val: value.clone() } )
        } else {
            Ok(TokenValue::Int(Int{number, size}))
        }
    }

    fn tokenize_cell(value: &Value) -> Result<TokenValue> {
        let string = value
            .as_str()
            .ok_or_else(|| AbiError::WrongDataFormat { val: value.clone() } )?;

        if string.len() == 0 {
            return Ok(TokenValue::Cell(BuilderData::new().into()));
        }

        let data = base64::decode(string)
            .map_err(|_| AbiError::InvalidParameterValue { val: value.clone() } )?;
        let cell = deserialize_tree_of_cells(&mut Cursor::new(data))
            .map_err(|_| AbiError::InvalidParameterValue { val: value.clone() } )?;
        Ok(TokenValue::Cell(cell))
    }

    fn tokenize_hashmap(key_type: &ParamType, value_type: &ParamType, map_value: &Value) -> Result<TokenValue> {
        if let Value::Object(map) = map_value {
            let mut new_map = HashMap::<String, TokenValue>::new();
            for (key, value) in map.iter() {
                let value = Self::tokenize_parameter(value_type, value)?;
                new_map.insert(key.to_string(), value);
            }
            Ok(TokenValue::Map(key_type.clone(), new_map))
        } else {
            fail!(AbiError::WrongDataFormat { val: map_value.clone() } )
        }
    }

    fn tokenize_bytes(value: &Value, size: Option<usize>) -> Result<TokenValue> {
        let string = value
            .as_str()
            .ok_or_else(|| AbiError::WrongDataFormat { val: value.clone() } )?;
        let mut data = hex::decode(string)
            .map_err(|_| AbiError::InvalidParameterValue { val: value.clone() } )?;
        match size {
            Some(size) => if data.len() >= size {
                data.truncate(size);
                Ok(TokenValue::FixedBytes(data))
            } else {
                fail!(AbiError::InvalidParameterValue { val: value.clone() } )
            }
            None => Ok(TokenValue::Bytes(data))
        }
    }

    /// Tries to parse a value as tuple.
    fn tokenize_tuple(params: &Vec<Param>, value: &Value) -> Result<TokenValue> {
        let tokens = Self::tokenize_all_params(params, value)?;

        Ok(TokenValue::Tuple(tokens))
    }

    /// Tries to parse a value as time.
    fn tokenize_time(value: &Value) -> Result<TokenValue> {
        let number = Self::read_uint(value)?;

        let time = number.to_u64().ok_or_else(|| error!(AbiError::InvalidInputData {
            msg: "`time` value should fit into u64".into()
        }))?;

        Ok(TokenValue::Time(time))
    }

    /// Tries to parse a value as expire.
    fn tokenize_expire(value: &Value) -> Result<TokenValue> {
        let number = Self::read_uint(value)?;

        let expire = number.to_u32().ok_or_else(|| error!(AbiError::InvalidInputData {
            msg: "`expire` value should fit into u32".into()
        }))?;

        Ok(TokenValue::Expire(expire))
    }

    fn tokenize_public_key(value: &Value) -> Result<TokenValue> {
        let string = value
            .as_str()
            .ok_or_else(|| AbiError::WrongDataFormat { val: value.clone() } )?;

        if string.len() == 0 {
            Ok(TokenValue::PublicKey(None))
        } else {
            let data = hex::decode(string)
                .map_err(|_| AbiError::InvalidParameterValue { val: value.clone() } )?;
            if data.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
                fail!(AbiError::InvalidParameterLength { val: value.clone() } )
            };
            Ok(TokenValue::PublicKey(Some(ed25519_dalek::PublicKey::from_bytes(&data)?)))
        }
    }
}
