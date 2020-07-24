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
    error::AbiError, int::{Int, Uint}, param::Param, param_type::ParamType,
    token::{Token, TokenValue}
};

use num_bigint::{BigInt, BigUint};
use serde_json;
use std::collections::HashMap;
use ton_block::{MsgAddress, types::Grams};
use ton_types::{
    BuilderData, Cell, error, fail, HashmapE, HashmapType, IBitstring, Result, SliceData
};

impl TokenValue {
    /// Deserializes value from `SliceData` to `TokenValue`
    pub fn read_from(param_type: &ParamType, mut cursor: SliceData, last: bool, abi_version: u8) -> Result<(Self, SliceData)> {
        match param_type {
            ParamType::Unknown => 
                fail!(AbiError::DeserializationError { msg: "Unknown ParamType", cursor } ),
            ParamType::Uint(size) => Self::read_uint(*size, cursor),
            ParamType::Int(size) => Self::read_int(*size, cursor),
            ParamType::Bool => {
                cursor = find_next_bits(cursor, 1)?;
                Ok((TokenValue::Bool(cursor.get_next_bit()?), cursor))
            }
            ParamType::Tuple(tuple_params) => Self::read_tuple(tuple_params, cursor, last, abi_version),
            ParamType::Array(param_type) => Self::read_array(&param_type, cursor, abi_version),
            ParamType::FixedArray(param_type, size) => {
                Self::read_fixed_array(&param_type, *size, cursor, abi_version)
            }
            ParamType::Cell => Self::read_cell(cursor, last, abi_version)
                .map(|(cell, cursor)| (TokenValue::Cell(cell), cursor)),
            ParamType::Map(key_type, value_type) => Self::read_hashmap(key_type, value_type, cursor, abi_version),
            ParamType::Address => {
                cursor = find_next_bits(cursor, 1)?;
                let address = <MsgAddress as ton_block::Deserializable>::construct_from(&mut cursor)?;
                Ok((TokenValue::Address(address), cursor))
            }
            ParamType::Bytes => Self::read_bytes(None, cursor, last, abi_version),
            ParamType::FixedBytes(size) => Self::read_bytes(Some(*size), cursor, last, abi_version),
            ParamType::Gram => {
                cursor = find_next_bits(cursor, 1)?;
                let gram = <Grams as ton_block::Deserializable>::construct_from(&mut cursor)?;
                Ok((TokenValue::Gram(gram), cursor))
            },
            ParamType::Time => Self::read_time(cursor),
            ParamType::Expire => Self::read_expire(cursor),
            ParamType::PublicKey => Self::read_public_key(cursor)
        }
    }

    fn read_uint(size: usize, cursor: SliceData) -> Result<(Self, SliceData)> {
        let (vec, cursor) = get_next_bits_from_chain(cursor, size)?;
        let number = BigUint::from_bytes_be(&vec) >> (vec.len() * 8 - size);
        Ok((TokenValue::Uint(Uint { number, size }), cursor))
    }

    fn read_int(size: usize, cursor: SliceData) -> Result<(Self, SliceData)> {
        let (vec, cursor) = get_next_bits_from_chain(cursor, size)?;
        let number = BigInt::from_signed_bytes_be(&vec) >> (vec.len() * 8 - size);
        Ok((TokenValue::Int(Int { number, size }), cursor))
    }

    fn read_tuple(tuple_params: &[Param], cursor: SliceData, last: bool, abi_version: u8) -> Result<(Self, SliceData)> {
        let mut tokens = Vec::new();
        let mut cursor = cursor;
        for param in tuple_params {
            let last = last && Some(param) == tuple_params.last();
            let (token_value, new_cursor) = TokenValue::read_from(&param.kind, cursor, last, abi_version)?;
            tokens.push(Token {
                name: param.name.clone(),
                value: token_value,
            });
            cursor = new_cursor;
        }
        Ok((TokenValue::Tuple(tokens), cursor))
    }

    fn read_array_from_map(param_type: &ParamType, mut cursor: SliceData, size: usize, abi_version: u8)
    -> Result<(Vec<Self>, SliceData)> {
        let original = cursor.clone();
        cursor = find_next_bits(cursor, 1)?;
        let map = HashmapE::with_data(32, cursor.get_dictionary()?);
        let mut result = vec![];
        for i in 0..size {
            let mut index = BuilderData::new();
            index.append_u32(i as u32)?;
            match map.get(index.into()) {
                Ok(Some(item_slice)) => {
                    let (token, item_slice) = Self::read_from(param_type, item_slice, true, abi_version)?;
                    if item_slice.remaining_references() != 0 || item_slice.remaining_bits() != 0 {
                        fail!(AbiError::IncompleteDeserializationError { cursor: original } )
                    }
                    result.push(token);
                }
                _ => fail!(AbiError::DeserializationError { msg: "", cursor: original } )
            }
        }

        Ok((result, cursor))
    }

    fn read_array(param_type: &ParamType, mut cursor: SliceData, abi_version: u8) -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 32)?;
        let size = cursor.get_next_u32()?;
        let (result, cursor) = Self::read_array_from_map(param_type, cursor, size as usize, abi_version)?;

        Ok((TokenValue::Array(result), cursor))
    }

    fn read_fixed_array(param_type: &ParamType, size: usize, cursor: SliceData, abi_version: u8) -> Result<(Self, SliceData)> {
        let (result, cursor) = Self::read_array_from_map(param_type, cursor, size, abi_version)?;

        Ok((TokenValue::FixedArray(result), cursor))
    }

    fn read_cell(mut cursor: SliceData, last: bool, abi_version: u8) -> Result<(Cell, SliceData)> {
        let cell = match cursor.remaining_references() {
            1 if (abi_version == 1 && cursor.cell().references_count() == BuilderData::references_capacity())
                || (abi_version != 1 && !last && cursor.remaining_bits() == 0) => {
                cursor = SliceData::from(cursor.reference(0)?);
                cursor.checked_drain_reference()?
            }
            _ => cursor.checked_drain_reference()?
        };
        Ok((cell.clone(), cursor))
    }

    fn read_hashmap(key_type: &ParamType, value_type: &ParamType, mut cursor: SliceData, abi_version: u8)
    -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 1)?;
        let mut new_map = HashMap::new();
        let bit_len = match key_type {
            ParamType::Int(size) | ParamType::Uint(size) => *size,
            ParamType::Address => super::STD_ADDRESS_BIT_LENGTH,
            _ => fail!(AbiError::InvalidData { msg: "Only integer and std address values can be map keys".to_owned() } )
        };
        let hashmap = HashmapE::with_data(bit_len, cursor.get_dictionary()?);
        hashmap.iterate(&mut |key, value| -> Result<bool> {
            let key = Self::read_from(key_type, key, true, abi_version)?.0;
            let key = serde_json::to_value(&key)?.as_str().ok_or(AbiError::InvalidData {
                msg: "Non-ordinary key".to_owned()
            })?.to_owned();
            let value = Self::read_from(value_type, value, true, abi_version)?.0;
            new_map.insert(key, value);
            Ok(true)
        })?;
        Ok((TokenValue::Map(key_type.clone(), new_map), cursor))
    }

    fn read_bytes(size: Option<usize>, cursor: SliceData, last: bool, abi_version: u8) -> Result<(Self, SliceData)> {
        let original = cursor.clone();
        let (mut cell, cursor) = Self::read_cell(cursor, last, abi_version)?;

        let mut data = vec![];
        loop {
            data.extend_from_slice(cell.data());
            data.pop();
            cell = match cell.reference(0) {
                Ok(cell) => cell.clone(),
                Err(_) => break
            };
        }
        match size {
            Some(size) if size == data.len() => Ok((TokenValue::FixedBytes(data), cursor)),
            Some(_) => fail!(AbiError::DeserializationError {
                msg: "Size of fixed bytes is not correspond to expected size",
                cursor: original
            }),
            None => Ok((TokenValue::Bytes(data), cursor))
        }
    }

    fn read_time(mut cursor: SliceData) -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 64)?;
        Ok((TokenValue::Time(cursor.get_next_u64()?), cursor))
    }

    fn read_expire(mut cursor: SliceData) -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 32)?;
        Ok((TokenValue::Expire(cursor.get_next_u32()?), cursor))
    }

    fn read_public_key(mut cursor: SliceData) -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 1)?;
        if cursor.get_next_bit()? {
            let (vec, cursor) = get_next_bits_from_chain(cursor, 256)?;
            Ok((TokenValue::PublicKey(Some(ed25519_dalek::PublicKey::from_bytes(&vec)?)), cursor))
        } else {
            Ok((TokenValue::PublicKey(None), cursor))
        }
    }

    /// Decodes provided params from SliceData
    pub fn decode_params(params: &Vec<Param>, mut cursor: SliceData, abi_version: u8) -> Result<Vec<Token>> {
        let mut tokens = vec![];

        for param in params {
            // println!("{:?}", param);
            let last = Some(param) == params.last();
            let (token_value, new_cursor) = Self::read_from(&param.kind, cursor, last, abi_version)?;

            cursor = new_cursor;
            tokens.push(Token { name: param.name.clone(), value: token_value });
        }

        if cursor.remaining_references() != 0 || cursor.remaining_bits() != 0 {
            fail!(AbiError::IncompleteDeserializationError { cursor })
        } else {
            Ok(tokens)
        }
    }
}

fn get_next_bits_from_chain(mut cursor: SliceData, bits: usize) -> Result<(Vec<u8>, SliceData)> {
    cursor = find_next_bits(cursor, bits)?;
    Ok((cursor.get_next_bits(bits)?, cursor))
}

fn find_next_bits(mut cursor: SliceData, bits: usize) -> Result<SliceData> {
    debug_assert!(bits != 0);
    let original = cursor.clone();
    if cursor.remaining_bits() == 0 {
        if cursor.reference(1).is_ok() {
            fail!(AbiError::IncompleteDeserializationError { cursor: original } )
        }
        cursor = cursor.reference(0)?.into();
    }
    match cursor.remaining_bits() >= bits  {
        true => Ok(cursor),
        false => fail!(AbiError::DeserializationError { 
            msg: "Not enought remaining bits in the cell", 
            cursor: original
        })
    }
}
