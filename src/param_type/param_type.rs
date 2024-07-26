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

//! Function and event param types.

use crate::{AbiError, Param};
use crate::contract::{AbiVersion, ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_1, ABI_VERSION_2_4};
use std::fmt;

use ever_block::{error, Result};

/// Function and event param types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
    /// uint<M>: unsigned integer type of M bits.
    Uint(usize),
    /// int<M>: signed integer type of M bits.
    Int(usize),
    /// varuint<M>: variable length unsigned integer type of maximum M bytes.
    VarUint(usize),
    /// varint<M>: variable length integer type of maximum M bytes.
    VarInt(usize),
    /// bool: boolean value.
    Bool,
    /// Tuple: several values combined into tuple.
    Tuple(Vec<Param>),
    /// T[]: dynamic array of elements of the type T.
    Array(Box<ParamType>),
    /// T[k]: dynamic array of elements of the type T.
    FixedArray(Box<ParamType>, usize),
    /// cell - tree of cells
    Cell,
    /// hashmap - values dictionary
    Map(Box<ParamType>, Box<ParamType>),
    /// message address
    Address,
    /// std address
    AddressStd,
    /// byte array
    Bytes,
    /// fixed size byte array
    FixedBytes(usize),
    /// UTF8 string
    String,
    /// Nanograms
    Token,
    /// Timestamp
    Time,
    /// Message expiration time
    Expire,
    /// Public key
    PublicKey,
    /// Optional parameter
    Optional(Box<ParamType>),
    /// Parameter stored in reference
    Ref(Box<ParamType>),
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.type_signature())
    }
}

impl ParamType {
    /// Returns type signature according to ABI specification
    pub fn type_signature(&self) -> String {
        match self {
            ParamType::Uint(size) => format!("uint{}", size),
            ParamType::Int(size) => format!("int{}", size),
            ParamType::VarUint(size) => format!("varuint{}", size),
            ParamType::VarInt(size) => format!("varint{}", size),
            ParamType::Bool => "bool".to_owned(),
            ParamType::Tuple(params) => {
                let mut signature = "".to_owned();
                for param in params {
                    signature += ",";
                    signature += &param.kind.type_signature();
                }
                signature.replace_range(..1, "(");
                signature + ")"
            }
            ParamType::Array(ref param_type) => format!("{}[]", param_type.type_signature()),
            ParamType::FixedArray(ref param_type, size) => {
                format!("{}[{}]", param_type.type_signature(), size)
            }
            ParamType::Cell => "cell".to_owned(),
            ParamType::Map(key_type, value_type) => format!(
                "map({},{})",
                key_type.type_signature(),
                value_type.type_signature()
            ),
            ParamType::Address => format!("address"),
            ParamType::AddressStd => format!("address_std"),
            ParamType::Bytes => format!("bytes"),
            ParamType::FixedBytes(size) => format!("fixedbytes{}", size),
            ParamType::String => format!("string"),
            ParamType::Token => format!("gram"),
            ParamType::Time => format!("time"),
            ParamType::Expire => format!("expire"),
            ParamType::PublicKey => format!("pubkey"),
            ParamType::Optional(ref param_type) => {
                format!("optional({})", param_type.type_signature())
            }
            ParamType::Ref(ref param_type) => format!("ref({})", param_type.type_signature()),
        }
    }

    pub fn set_components(&mut self, components: Vec<Param>) -> Result<()> {
        match self {
            ParamType::Tuple(params) => {
                if components.len() == 0 {
                    Err(error!(AbiError::EmptyComponents))
                } else {
                    Ok(*params = components)
                }
            }
            ParamType::Array(array_type) => array_type.set_components(components),
            ParamType::FixedArray(array_type, _) => array_type.set_components(components),
            ParamType::Map(_, value_type) => value_type.set_components(components),
            ParamType::Optional(inner_type) => inner_type.set_components(components),
            ParamType::Ref(inner_type) => inner_type.set_components(components),
            _ => {
                if components.len() != 0 {
                    Err(error!(AbiError::UnusedComponents))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Check if parameter type is supoorted in particular ABI version
    pub fn is_supported(&self, abi_version: &AbiVersion) -> bool {
        match self {
            ParamType::Time | ParamType::Expire | ParamType::PublicKey => {
                abi_version >= &ABI_VERSION_2_0
            }
            ParamType::String
            | ParamType::Optional(_)
            | ParamType::VarInt(_)
            | ParamType::VarUint(_) => abi_version >= &ABI_VERSION_2_1,
            ParamType::Ref(_) => abi_version >= &ABI_VERSION_2_4,
            _ => abi_version >= &ABI_VERSION_1_0,
        }
    }
}
