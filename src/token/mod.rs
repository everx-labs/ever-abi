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

//! TON ABI params.
use crate::{
    error::AbiError,
    int::{Int, Uint},
    param::Param,
    param_type::ParamType,
};

use chrono::prelude::Utc;
use num_bigint::{BigInt, BigUint};
use std::collections::BTreeMap;
use std::fmt;
use ton_block::{Grams, MsgAddress};
use ton_types::{Cell, Result};

mod deserialize;
mod detokenizer;
mod serialize;
mod tokenizer;

pub use self::deserialize::*;
pub use self::detokenizer::*;
pub use self::serialize::*;
pub use self::tokenizer::*;

#[cfg(test)]
mod test_encoding;
#[cfg(test)]
mod tests;

pub const STD_ADDRESS_BIT_LENGTH: usize = 267;
pub const MAX_HASH_MAP_INFO_ABOUT_KEY: usize = 12;

/// TON ABI params.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub name: String,
    pub value: TokenValue,
}

impl Token {
    pub fn new(name: &str, value: TokenValue) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} : {}", self.name, self.value)
    }
}

/// TON ABI param values.
#[derive(Debug, PartialEq, Clone)]
pub enum TokenValue {
    /// uint<M>: unsigned integer type of M bits.
    ///
    /// Encoded as M bits of big-endian number representation put into cell data.
    Uint(Uint),
    /// int<M>: signed integer type of M bits.
    ///
    /// Encoded as M bits of big-endian number representation put into cell data.
    Int(Int),
    /// Variable length integer
    ///
    /// Encoded according to blockchain specification
    VarInt(usize, BigInt),
    /// Variable length unsigned integer
    ///
    /// Encoded according to blockchain specification
    VarUint(usize, BigUint),
    /// bool: boolean value.
    ///
    /// Encoded as one bit put into cell data.
    Bool(bool),
    /// Tuple: several values combinde into tuple.
    ///
    /// Encoded as all tuple elements encodings put into cell data one by one.
    Tuple(Vec<Token>),
    /// T[]: dynamic array of elements of the type T.
    ///
    /// Encoded as all array elements encodings put to separate cell.
    Array(ParamType, Vec<TokenValue>),
    /// T[k]: dynamic array of elements of the type T.
    ///
    /// Encoded as all array elements encodings put to separate cell.
    FixedArray(ParamType, Vec<TokenValue>),
    /// TVM Cell
    ///
    Cell(Cell),
    /// Dictionary of values
    ///
    Map(ParamType, ParamType, BTreeMap<String, TokenValue>),
    /// MsgAddress
    ///
    Address(MsgAddress),
    /// Raw byte array
    ///
    /// Encoded as separate cells chain
    Bytes(Vec<u8>),
    /// Fixed sized raw byte array
    ///
    /// Encoded as separate cells chain
    FixedBytes(Vec<u8>),
    /// UTF8 string
    ///
    /// Encoded similar to `Bytes`
    String(String),
    /// Nanograms
    ///
    Token(Grams),
    /// Timestamp
    Time(u64),
    /// Message expiration time
    Expire(u32),
    /// Public key
    PublicKey(Option<ed25519_dalek::PublicKey>),
    /// Optional parameter
    Optional(ParamType, Option<Box<TokenValue>>),
    /// Parameter stored in reference
    Ref(Box<TokenValue>),
}

impl fmt::Display for TokenValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenValue::Uint(u) => write!(f, "{}", u.number),
            TokenValue::Int(u) => write!(f, "{}", u.number),
            TokenValue::VarUint(_, u) => write!(f, "{}", u),
            TokenValue::VarInt(_, u) => write!(f, "{}", u),
            TokenValue::Bool(b) => write!(f, "{}", b),
            TokenValue::Tuple(ref arr) => {
                let s = arr
                    .iter()
                    .map(|ref t| format!("{}", t))
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "({})", s)
            }
            TokenValue::Array(_, ref arr) | TokenValue::FixedArray(_, ref arr) => {
                let s = arr
                    .iter()
                    .map(|ref t| format!("{}", t))
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "[{}]", s)
            }
            TokenValue::Cell(c) => write!(f, "{:?}", c),
            TokenValue::Map(_key_type, _value_type, map) => {
                let s = map
                    .iter()
                    .map(|ref t| format!("{}:{}", t.0, t.1))
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "{{{}}}", s)
            }
            TokenValue::Address(a) => write!(f, "{}", a),
            TokenValue::Bytes(ref arr) | TokenValue::FixedBytes(ref arr) => write!(f, "{:?}", arr),
            TokenValue::String(string) => write!(f, "{}", string),
            TokenValue::Token(g) => write!(f, "{}", g),
            TokenValue::Time(time) => write!(f, "{}", time),
            TokenValue::Expire(expire) => write!(f, "{}", expire),
            TokenValue::Ref(value) => write!(f, "{}", value),
            TokenValue::PublicKey(key) => {
                if let Some(key) = key {
                    write!(f, "{}", hex::encode(key.to_bytes()))
                } else {
                    write!(f, "None")
                }
            }
            TokenValue::Optional(_, value) => {
                if let Some(value) = value {
                    write!(f, "{}", value)
                } else {
                    write!(f, "None")
                }
            }
        }
    }
}

impl TokenValue {
    /// Check whether the type of the token matches the given parameter type.
    ///
    /// Numeric types (`Int` and `Uint`) type check if the size of the token
    /// type is of equal size with the provided parameter type.
    pub fn type_check(&self, param_type: &ParamType) -> bool {
        match self {
            TokenValue::Uint(uint) => *param_type == ParamType::Uint(uint.size),
            TokenValue::Int(int) => *param_type == ParamType::Int(int.size),
            TokenValue::VarUint(size, _) => *param_type == ParamType::VarUint(*size),
            TokenValue::VarInt(size, _) => *param_type == ParamType::VarInt(*size),
            TokenValue::Bool(_) => *param_type == ParamType::Bool,
            TokenValue::Tuple(ref arr) => {
                if let ParamType::Tuple(ref params) = *param_type {
                    Token::types_check(arr, params)
                } else {
                    false
                }
            }
            TokenValue::Array(inner_type, ref tokens) => {
                if let ParamType::Array(ref param_type) = *param_type {
                    inner_type == param_type.as_ref()
                        && tokens.iter().all(|t| t.type_check(param_type))
                } else {
                    false
                }
            }
            TokenValue::FixedArray(inner_type, ref tokens) => {
                if let ParamType::FixedArray(ref param_type, size) = *param_type {
                    size == tokens.len()
                        && inner_type == param_type.as_ref()
                        && tokens.iter().all(|t| t.type_check(param_type))
                } else {
                    false
                }
            }
            TokenValue::Cell(_) => *param_type == ParamType::Cell,
            TokenValue::Map(map_key_type, map_value_type, ref values) => {
                if let ParamType::Map(ref key_type, ref value_type) = *param_type {
                    map_key_type == key_type.as_ref()
                        && map_value_type == value_type.as_ref()
                        && values.iter().all(|t| t.1.type_check(value_type))
                } else {
                    false
                }
            }
            TokenValue::Address(_) => *param_type == ParamType::Address,
            TokenValue::Bytes(_) => *param_type == ParamType::Bytes,
            TokenValue::FixedBytes(ref arr) => *param_type == ParamType::FixedBytes(arr.len()),
            TokenValue::String(_) => *param_type == ParamType::String,
            TokenValue::Token(_) => *param_type == ParamType::Token,
            TokenValue::Time(_) => *param_type == ParamType::Time,
            TokenValue::Expire(_) => *param_type == ParamType::Expire,
            TokenValue::PublicKey(_) => *param_type == ParamType::PublicKey,
            TokenValue::Optional(opt_type, opt_value) => {
                if let ParamType::Optional(ref param_type) = *param_type {
                    param_type.as_ref() == opt_type
                        && opt_value
                            .as_ref()
                            .map(|val| val.type_check(param_type))
                            .unwrap_or(true)
                } else {
                    false
                }
            }
            TokenValue::Ref(value) => {
                if let ParamType::Ref(ref param_type) = *param_type {
                    value.type_check(param_type)
                } else {
                    false
                }
            }
        }
    }

    /// Returns `ParamType` the token value represents
    pub(crate) fn get_param_type(&self) -> ParamType {
        match self {
            TokenValue::Uint(uint) => ParamType::Uint(uint.size),
            TokenValue::Int(int) => ParamType::Int(int.size),
            TokenValue::VarUint(size, _) => ParamType::VarUint(*size),
            TokenValue::VarInt(size, _) => ParamType::VarInt(*size),
            TokenValue::Bool(_) => ParamType::Bool,
            TokenValue::Tuple(ref arr) => {
                ParamType::Tuple(arr.iter().map(|token| token.get_param()).collect())
            }
            TokenValue::Array(param_type, _) => ParamType::Array(Box::new(param_type.clone())),
            TokenValue::FixedArray(param_type, tokens) => {
                ParamType::FixedArray(Box::new(param_type.clone()), tokens.len())
            }
            TokenValue::Cell(_) => ParamType::Cell,
            TokenValue::Map(key_type, value_type, _) => {
                ParamType::Map(Box::new(key_type.clone()), Box::new(value_type.clone()))
            }
            TokenValue::Address(_) => ParamType::Address,
            TokenValue::Bytes(_) => ParamType::Bytes,
            TokenValue::FixedBytes(ref arr) => ParamType::FixedBytes(arr.len()),
            TokenValue::String(_) => ParamType::String,
            TokenValue::Token(_) => ParamType::Token,
            TokenValue::Time(_) => ParamType::Time,
            TokenValue::Expire(_) => ParamType::Expire,
            TokenValue::PublicKey(_) => ParamType::PublicKey,
            TokenValue::Optional(ref param_type, _) => {
                ParamType::Optional(Box::new(param_type.clone()))
            }
            TokenValue::Ref(value) => ParamType::Ref(Box::new(value.get_param_type())),
        }
    }

    pub fn get_default_value_for_header(param_type: &ParamType) -> Result<Self> {
        match param_type {
            ParamType::Time => Ok(TokenValue::Time(Utc::now().timestamp_millis() as u64)),
            ParamType::Expire => Ok(TokenValue::Expire(u32::max_value())),
            ParamType::PublicKey => Ok(TokenValue::PublicKey(None)),
            any_type => Err(AbiError::InvalidInputData {
                msg: format!(
                    "Type {} doesn't have default value and must be explicitly defined",
                    any_type
                ),
            }
            .into()),
        }
    }
}

impl Token {
    /// Check if all the types of the tokens match the given parameter types.
    pub fn types_check(tokens: &[Token], params: &[Param]) -> bool {
        params.len() == tokens.len() && {
            params.iter().zip(tokens).all(|(param, token)| {
                // println!("{} {} {}", token.name, token.value, param.kind);
                token.value.type_check(&param.kind) && token.name == param.name
            })
        }
    }

    /// Returns `Param` the token represents
    pub(crate) fn get_param(&self) -> Param {
        Param {
            name: self.name.clone(),
            kind: self.value.get_param_type(),
        }
    }
}
