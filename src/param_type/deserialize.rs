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

use crate::{error::AbiError, param_type::ParamType};
use serde::de::{Error as SerdeError, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use ever_block::{fail, Result};

impl<'a> Deserialize<'a> for ParamType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_identifier(ParamTypeVisitor)
    }
}

struct ParamTypeVisitor;

impl<'a> Visitor<'a> for ParamTypeVisitor {
    type Value = ParamType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a correct name of abi-encodable parameter type")
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
    where
        E: SerdeError,
    {
        read_type(value).map_err(|e| SerdeError::custom(e.to_string()))
    }

    fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.visit_str(value.as_str())
    }
}

/// Converts string to param type.
pub fn read_type(name: &str) -> Result<ParamType> {
    // check if it is a fixed or dynamic array.
    if let Some(']') = name.chars().last() {
        // take number part
        let num: String = name
            .chars()
            .rev()
            .skip(1)
            .take_while(|c| *c != '[')
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        let count = name.chars().count();
        if num.is_empty() {
            // we already know it's a dynamic array!
            let subtype = read_type(&name[..count - 2])?;
            return Ok(ParamType::Array(Box::new(subtype)));
        } else {
            // it's a fixed array.
            let len = usize::from_str_radix(&num, 10).map_err(|_| AbiError::InvalidName {
                name: name.to_owned(),
            })?;

            let subtype = read_type(&name[..count - num.len() - 2])?;
            return Ok(ParamType::FixedArray(Box::new(subtype), len));
        }
    }

    let result = match name {
        "bool" => ParamType::Bool,
        // a little trick - here we only recognize parameter as a tuple and fill it
        // with parameters in `Param` type deserialization
        "tuple" => ParamType::Tuple(Vec::new()),
        s if s.starts_with("int") => {
            let len = usize::from_str_radix(&s[3..], 10).map_err(|_| AbiError::InvalidName {
                name: name.to_owned(),
            })?;
            ParamType::Int(len)
        }
        s if s.starts_with("uint") => {
            let len = usize::from_str_radix(&s[4..], 10).map_err(|_| AbiError::InvalidName {
                name: name.to_owned(),
            })?;
            ParamType::Uint(len)
        }
        s if s.starts_with("varint") => {
            let len = usize::from_str_radix(&s[6..], 10).map_err(|_| AbiError::InvalidName {
                name: name.to_owned(),
            })?;
            ParamType::VarInt(len)
        }
        s if s.starts_with("varuint") => {
            let len = usize::from_str_radix(&s[7..], 10).map_err(|_| AbiError::InvalidName {
                name: name.to_owned(),
            })?;
            ParamType::VarUint(len)
        }
        s if s.starts_with("map(") && s.ends_with(")") => {
            let types: Vec<&str> = name[4..name.len() - 1].splitn(2, ",").collect();
            if types.len() != 2 {
                fail!(AbiError::InvalidName {
                    name: name.to_owned()
                });
            }

            let key_type = read_type(types[0])?;
            let value_type = read_type(types[1])?;

            match key_type {
                ParamType::Int(_) | ParamType::Uint(_) | ParamType::Address => {
                    ParamType::Map(Box::new(key_type), Box::new(value_type))
                }
                _ => fail!(AbiError::InvalidName {
                    name: "Only integer and std address values can be map keys".to_owned()
                }),
            }
        }
        "cell" => ParamType::Cell,
        "address" => ParamType::Address,
        "token" => ParamType::Token,
        "bytes" => ParamType::Bytes,
        s if s.starts_with("fixedbytes") => {
            let len = usize::from_str_radix(&s[10..], 10).map_err(|_| AbiError::InvalidName {
                name: name.to_owned(),
            })?;
            ParamType::FixedBytes(len)
        }
        "time" => ParamType::Time,
        "expire" => ParamType::Expire,
        "pubkey" => ParamType::PublicKey,
        "string" => ParamType::String,
        s if s.starts_with("optional(") && s.ends_with(")") => {
            let inner_type = read_type(&name[9..name.len() - 1])?;
            ParamType::Optional(Box::new(inner_type))
        }
        s if s.starts_with("ref(") && s.ends_with(")") => {
            let inner_type = read_type(&name[4..name.len() - 1])?;
            ParamType::Ref(Box::new(inner_type))
        }
        _ => {
            fail!(AbiError::InvalidName {
                name: name.to_owned()
            });
        }
    };

    Ok(result)
}
