/*
* Copyright (C) 2019-2022 TON Labs. All Rights Reserved.
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
    error::AbiError,
    int::{Int, Uint},
    param::Param,
    param_type::ParamType,
    token::{Token, TokenValue},
};

use num_bigint::{BigInt, BigUint, Sign};
use num_traits::cast::ToPrimitive;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    io::Cursor,
    str::FromStr,
};
use ton_block::Grams;
use ton_types::{deserialize_tree_of_cells, error, fail, Cell, Result};

/// This struct should be used to parse string values as tokens.
pub struct Tokenizer;

impl Tokenizer {
    /// Tries to parse a JSON value as a token of given type.
    pub fn tokenize_parameter(param: &ParamType, value: &Value, name: &str) -> Result<TokenValue> {
        match &param {
            ParamType::Uint(size) => Self::tokenize_uint(*size, value, name),
            ParamType::Int(size) => Self::tokenize_int(*size, value, name),
            ParamType::VarUint(size) => Self::tokenize_varuint(*size, value, name),
            ParamType::VarInt(size) => Self::tokenize_varint(*size, value, name),
            ParamType::Bool => Self::tokenize_bool(value, name),
            ParamType::Tuple(tuple_params) => Self::tokenize_tuple(tuple_params, value, name),
            ParamType::Array(param_type) => Self::tokenize_array(param_type, value, name),
            ParamType::FixedArray(param_type, size) => {
                Self::tokenize_fixed_array(param_type, *size, value, name)
            }
            ParamType::Cell => Self::tokenize_cell(value, name),
            ParamType::Map(key_type, value_type) => {
                Self::tokenize_hashmap(key_type, value_type, value, name)
            }
            ParamType::Address => Self::tokenize_address(value, name),
            ParamType::Bytes => Self::tokenize_bytes(value, None, name),
            ParamType::FixedBytes(size) => Self::tokenize_bytes(value, Some(*size), name),
            ParamType::String => Self::tokenize_string(value, name),
            ParamType::Token => Self::tokenize_gram(value, name),
            ParamType::Time => Self::tokenize_time(value, name),
            ParamType::Expire => Self::tokenize_expire(value, name),
            ParamType::PublicKey => Self::tokenize_public_key(value, name),
            ParamType::Optional(param_type) => Self::tokenize_optional(param_type, value, name),
            ParamType::Ref(param_type) => Self::tokenize_ref(param_type, value, name),
        }
    }

    /// Tries to parse parameters from JSON values to tokens.
    pub fn tokenize_all_params(params: &[Param], values: &Value) -> Result<Vec<Token>> {
        if let Value::Object(map) = values {
            let mut tokens = Vec::new();
            for param in params {
                let value = map.get(&param.name).unwrap_or(&Value::Null);
                let token_value = Self::tokenize_parameter(&param.kind, value, &param.name)?;
                tokens.push(Token {
                    name: param.name.clone(),
                    value: token_value,
                });
            }

            Ok(tokens)
        } else {
            fail!(AbiError::InvalidInputData {
                msg: "Contract function parameters should be passed as a JSON object".to_string()
            })
        }
    }

    /// Tries to parse parameters from JSON values to tokens.
    pub fn tokenize_optional_params(
        params: &[Param],
        values: &Value,
        default_values: &HashMap<String, TokenValue>,
    ) -> Result<HashMap<String, TokenValue>> {
        if let Value::Object(map) = values {
            let mut map = map.clone();
            let mut tokens = HashMap::new();
            for param in params {
                if let Some(value) = map.remove(&param.name) {
                    let token_value = Self::tokenize_parameter(&param.kind, &value, &param.name)?;
                    tokens.insert(param.name.clone(), token_value);
                } else if let Some(value) = default_values.get(&param.name) {
                    tokens.insert(param.name.clone(), value.clone());
                }
            }
            if !map.is_empty() {
                let unknown = map
                    .iter()
                    .map(|(key, _)| key.as_ref())
                    .collect::<Vec<&str>>()
                    .join(", ");
                return Err(AbiError::InvalidInputData {
                    msg: format!("Contract doesn't have following parameters: {}", unknown),
                }
                .into());
            }
            Ok(tokens)
        } else {
            fail!(AbiError::InvalidInputData {
                msg: "Contract function parameters should be passed as a JSON object".to_string()
            })
        }
    }

    /// Tries to read tokens array from `Value`
    fn read_array(item_type: &ParamType, value: &Value, name: &str) -> Result<Vec<TokenValue>> {
        if let Value::Array(array) = value {
            let mut tokens = Vec::new();
            for value in array {
                tokens.push(Self::tokenize_parameter(item_type, value, name)?);
            }

            Ok(tokens)
        } else {
            fail!(AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "array".to_string()
            })
        }
    }

    /// Tries to parse a value as a vector of tokens of fixed size.
    fn tokenize_fixed_array(
        item_type: &ParamType,
        size: usize,
        value: &Value,
        name: &str,
    ) -> Result<TokenValue> {
        let vec = Self::read_array(item_type, value, name)?;
        match vec.len() == size {
            true => Ok(TokenValue::FixedArray(item_type.clone(), vec)),
            false => fail!(AbiError::InvalidParameterLength {
                val: value.clone(),
                name: name.to_string(),
                expected: format!("array of {} elements", size),
            }),
        }
    }

    /// Tries to parse a value as a vector of tokens.
    fn tokenize_array(item_type: &ParamType, value: &Value, name: &str) -> Result<TokenValue> {
        let vec = Self::read_array(item_type, value, name)?;

        Ok(TokenValue::Array(item_type.clone(), vec))
    }

    /// Tries to parse a value as a bool.
    fn tokenize_bool(value: &Value, name: &str) -> Result<TokenValue> {
        match value {
            Value::Bool(value) => Ok(TokenValue::Bool(value.to_owned())),
            Value::String(string) => match string.as_str() {
                "true" => Ok(TokenValue::Bool(true)),
                "false" => Ok(TokenValue::Bool(false)),
                _ => fail!(AbiError::InvalidParameterValue {
                    val: value.clone(),
                    name: name.to_string(),
                    err: "string should contain `true` or `false`".to_string()
                }),
            },
            _ => fail!(AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "bool or string `true`/`false`".to_string()
            }),
        }
    }

    /// Tries to read integer number from `Value`
    fn read_int(value: &Value, name: &str) -> Result<BigInt> {
        if let Some(number) = value.as_i64() {
            Ok(BigInt::from(number))
        } else if let Some(string) = value.as_str() {
            let result = if string.starts_with("-0x") {
                BigInt::parse_bytes(&string.as_bytes()[3..], 16).map(|number| -number)
            } else if string.starts_with("0x") {
                BigInt::parse_bytes(&string.as_bytes()[2..], 16)
            } else {
                BigInt::parse_bytes(string.as_bytes(), 10)
            };
            match result {
                Some(number) => Ok(number),
                None => fail!(AbiError::InvalidParameterValue {
                    val: value.clone(),
                    name: name.to_string(),
                    err: "can not parse number from string".to_string()
                }),
            }
        } else {
            fail!(AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "number or string with encoded number".to_string()
            })
        }
    }

    /// Tries to read integer number from `Value`
    fn read_uint(value: &Value, name: &str) -> Result<BigUint> {
        if let Some(number) = value.as_u64() {
            Ok(BigUint::from(number))
        } else if let Some(string) = value.as_str() {
            let result = if let Some(stripped) = string.strip_prefix("0x") {
                BigUint::parse_bytes(stripped.as_bytes(), 16)
            } else {
                BigUint::parse_bytes(string.as_bytes(), 10)
            };
            match result {
                Some(number) => Ok(number),
                None => fail!(AbiError::InvalidParameterValue {
                    val: value.clone(),
                    name: name.to_string(),
                    err: "can not parse number from string".to_string()
                }),
            }
        } else {
            fail!(AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "number or string with encoded number".to_string()
            })
        }
    }

    /// Tries to read grams from `Value`
    fn read_grams(value: &Value, name: &str) -> Result<Grams> {
        if let Some(number) = value.as_u64() {
            Ok(Grams::from(number))
        } else if let Some(string) = value.as_str() {
            Grams::from_str(string).map_err(|_| {
                error!(AbiError::InvalidParameterValue {
                    val: value.clone(),
                    name: name.to_string(),
                    err: "can not parse number from string".to_string()
                })
            })
        } else {
            fail!(AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "number or string with encoded number".to_string()
            })
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
            (number.bits() as usize) <= size
        } else {
            (number.bits() as usize) < size
        }
    }

    /// Checks if given number can be fit into given bits count
    fn check_uint_size(number: &BigUint, size: usize) -> bool {
        (number.bits() as usize) < size
    }

    /// Tries to parse a value as grams.
    fn tokenize_gram(value: &Value, name: &str) -> Result<TokenValue> {
        let number = Self::read_grams(value, name)?;
        Ok(TokenValue::Token(number))
    }

    /// Tries to parse a value as unsigned integer.
    fn tokenize_uint(size: usize, value: &Value, name: &str) -> Result<TokenValue> {
        let number = Self::read_uint(value, name)?;

        if !Self::check_uint_size(&number, size + 1) {
            fail!(AbiError::InvalidParameterValue {
                val: value.clone(),
                name: name.to_string(),
                err: "provided number is out of type range".to_string()
            })
        } else {
            Ok(TokenValue::Uint(Uint { number, size }))
        }
    }

    /// Tries to parse a value as signed integer.
    fn tokenize_int(size: usize, value: &Value, name: &str) -> Result<TokenValue> {
        let number = Self::read_int(value, name)?;

        if !Self::check_int_size(&number, size) {
            fail!(AbiError::InvalidParameterValue {
                val: value.clone(),
                name: name.to_string(),
                err: "provided number is out of type range".to_string()
            })
        } else {
            Ok(TokenValue::Int(Int { number, size }))
        }
    }

    fn tokenize_varuint(size: usize, value: &Value, name: &str) -> Result<TokenValue> {
        let number = Self::read_uint(value, name)?;

        if !Self::check_uint_size(&number, (size - 1) * 8) {
            fail!(AbiError::InvalidParameterValue {
                val: value.clone(),
                name: name.to_string(),
                err: "provided number is out of type range".to_string()
            })
        } else {
            Ok(TokenValue::VarUint(size, number))
        }
    }

    fn tokenize_varint(size: usize, value: &Value, name: &str) -> Result<TokenValue> {
        let number = Self::read_int(value, name)?;

        if !Self::check_int_size(&number, (size - 1) * 8) {
            fail!(AbiError::InvalidParameterValue {
                val: value.clone(),
                name: name.to_string(),
                err: "provided number is out of type range".to_string()
            })
        } else {
            Ok(TokenValue::VarInt(size, number))
        }
    }

    fn tokenize_cell(value: &Value, name: &str) -> Result<TokenValue> {
        let string = value.as_str().ok_or_else(|| AbiError::WrongDataFormat {
            val: value.clone(),
            name: name.to_string(),
            expected: "base64-encoded cell BOC".to_string(),
        })?;

        if string.is_empty() {
            return Ok(TokenValue::Cell(Cell::default()));
        }

        let data = base64::decode(string).map_err(|err| AbiError::InvalidParameterValue {
            val: value.clone(),
            name: name.to_string(),
            err: format!("can not decode base64: {}", err),
        })?;
        let cell = deserialize_tree_of_cells(&mut Cursor::new(data)).map_err(|err| {
            AbiError::InvalidParameterValue {
                val: value.clone(),
                name: name.to_string(),
                err: format!("can not deserialize cell: {}", err),
            }
        })?;
        Ok(TokenValue::Cell(cell))
    }

    fn tokenize_hashmap(
        key_type: &ParamType,
        value_type: &ParamType,
        map_value: &Value,
        name: &str,
    ) -> Result<TokenValue> {
        if let Value::Object(map) = map_value {
            let mut new_map = BTreeMap::<String, TokenValue>::new();
            for (key, value) in map.iter() {
                let value = Self::tokenize_parameter(value_type, value, name)?;
                new_map.insert(key.to_string(), value);
            }
            Ok(TokenValue::Map(
                key_type.clone(),
                value_type.clone(),
                new_map,
            ))
        } else {
            fail!(AbiError::WrongDataFormat {
                val: map_value.clone(),
                name: name.to_string(),
                expected: "JSON object".to_string()
            })
        }
    }

    fn tokenize_bytes(value: &Value, size: Option<usize>, name: &str) -> Result<TokenValue> {
        let string = value.as_str().ok_or_else(|| AbiError::WrongDataFormat {
            val: value.clone(),
            name: name.to_string(),
            expected: "hex-encoded string".to_string(),
        })?;
        let mut data = hex::decode(string).map_err(|err| AbiError::InvalidParameterValue {
            val: value.clone(),
            name: name.to_string(),
            err: format!("can not decode hex: {}", err),
        })?;
        match size {
            Some(size) => {
                if data.len() >= size {
                    data.truncate(size);
                    Ok(TokenValue::FixedBytes(data))
                } else {
                    fail!(AbiError::InvalidParameterLength {
                        val: value.clone(),
                        name: name.to_string(),
                        expected: format!("{} bytes", size),
                    })
                }
            }
            None => Ok(TokenValue::Bytes(data)),
        }
    }

    fn tokenize_string(value: &Value, name: &str) -> Result<TokenValue> {
        let string = value
            .as_str()
            .ok_or_else(|| AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "string".to_string(),
            })?
            .to_owned();
        Ok(TokenValue::String(string))
    }

    /// Tries to parse a value as tuple.
    fn tokenize_tuple(params: &[Param], value: &Value, name: &str) -> Result<TokenValue> {
        if !value.is_object() {
            fail!(AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "JSON object".to_string()
            })
        }

        let tokens = Self::tokenize_all_params(params, value)?;

        Ok(TokenValue::Tuple(tokens))
    }

    /// Tries to parse a value as time.
    fn tokenize_time(value: &Value, name: &str) -> Result<TokenValue> {
        let number = Self::read_uint(value, name)?;

        let time = number.to_u64().ok_or_else(|| {
            error!(AbiError::InvalidInputData {
                msg: format!("`{}` value {} should fit into u64", name, number)
            })
        })?;

        Ok(TokenValue::Time(time))
    }

    /// Tries to parse a value as expire.
    fn tokenize_expire(value: &Value, name: &str) -> Result<TokenValue> {
        let number = Self::read_uint(value, name)?;

        let expire = number.to_u32().ok_or_else(|| {
            error!(AbiError::InvalidInputData {
                msg: format!("`{}` value {} should fit into u32", name, number)
            })
        })?;

        Ok(TokenValue::Expire(expire))
    }

    fn tokenize_public_key(value: &Value, name: &str) -> Result<TokenValue> {
        let string = value.as_str().ok_or_else(|| AbiError::WrongDataFormat {
            val: value.clone(),
            name: name.to_string(),
            expected: "hex-encoded string".to_string(),
        })?;

        if string.is_empty() {
            Ok(TokenValue::PublicKey(None))
        } else {
            let data = hex::decode(string).map_err(|err| AbiError::InvalidParameterValue {
                val: value.clone(),
                name: name.to_string(),
                err: format!("can not decode hex: {}", err),
            })?;
            if data.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
                fail!(AbiError::InvalidParameterLength {
                    val: value.clone(),
                    name: name.to_string(),
                    expected: format!("{} bytes", ed25519_dalek::PUBLIC_KEY_LENGTH),
                })
            };
            Ok(TokenValue::PublicKey(Some(
                ed25519_dalek::PublicKey::from_bytes(&data)?,
            )))
        }
    }

    fn tokenize_optional(inner_type: &ParamType, value: &Value, name: &str) -> Result<TokenValue> {
        if value.is_null() {
            Ok(TokenValue::Optional(inner_type.clone(), None))
        } else {
            Ok(TokenValue::Optional(
                inner_type.clone(),
                Some(Box::new(Self::tokenize_parameter(inner_type, value, name)?)),
            ))
        }
    }

    fn tokenize_ref(inner_type: &ParamType, value: &Value, name: &str) -> Result<TokenValue> {
        Ok(TokenValue::Ref(Box::new(Self::tokenize_parameter(
            inner_type, value, name,
        )?)))
    }

    fn tokenize_address(value: &Value, name: &str) -> Result<TokenValue> {
        let address = value
            .as_str()
            .ok_or_else(|| AbiError::WrongDataFormat {
                val: value.clone(),
                name: name.to_string(),
                expected: "address string".to_string(),
            })?
            .parse()
            .map_err(|err| AbiError::InvalidParameterValue {
                val: value.clone(),
                name: name.to_string(),
                err: format!("can not parse address: {}", err),
            })?;
        Ok(TokenValue::Address(address))
    }
}
