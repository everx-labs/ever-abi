/*
* Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
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

use crate::{contract::{ABI_VERSION_1_0, AbiVersion}, error::AbiError, int::{Int, Uint}, param::Param, param_type::ParamType, token::{Token, TokenValue}};

use num_bigint::{BigInt, BigUint};
use num_traits::ToPrimitive;
use serde_json;
use std::collections::BTreeMap;
use ton_block::{MsgAddress, types::Grams};
use ton_types::{
    BuilderData, Cell, error, fail, HashmapE, HashmapType, IBitstring, Result, SliceData
};

impl TokenValue {
    /// Deserializes value from `SliceData` to `TokenValue`
    pub fn read_from(
        param_type: &ParamType, mut cursor: SliceData, last: bool, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Self, SliceData)> {
        match param_type {
            ParamType::Uint(size) => Self::read_uint(*size, cursor),
            ParamType::Int(size) => Self::read_int(*size, cursor),
            ParamType::VarUint(size) => Self::read_varuint(*size, cursor),
            ParamType::VarInt(size) => Self::read_varint(*size, cursor),
            ParamType::Bool => {
                cursor = find_next_bits(cursor, 1)?;
                Ok((TokenValue::Bool(cursor.get_next_bit()?), cursor))
            }
            ParamType::Tuple(tuple_params) =>
                Self::read_tuple(tuple_params, cursor, last, abi_version, allow_partial),
            ParamType::Array(item_type) => Self::read_array(&item_type, cursor, abi_version, allow_partial),
            ParamType::FixedArray(item_type, size) => {
                Self::read_fixed_array(&item_type, *size, cursor, abi_version, allow_partial)
            }
            ParamType::Cell => Self::read_cell(cursor, last, abi_version)
                .map(|(cell, cursor)| (TokenValue::Cell(cell), cursor)),
            ParamType::Map(key_type, value_type) => 
                Self::read_hashmap(key_type, value_type, cursor, abi_version, allow_partial),
            ParamType::Address => {
                cursor = find_next_bits(cursor, 1)?;
                let address = <MsgAddress as ton_block::Deserializable>::construct_from(&mut cursor)?;
                Ok((TokenValue::Address(address), cursor))
            }
            ParamType::Bytes => Self::read_bytes(None, cursor, last, abi_version),
            ParamType::FixedBytes(size) => Self::read_bytes(Some(*size), cursor, last, abi_version),
            ParamType::String => Self::read_string(cursor, last, abi_version),
            ParamType::Token => {
                cursor = find_next_bits(cursor, 1)?;
                let gram = <Grams as ton_block::Deserializable>::construct_from(&mut cursor)?;
                Ok((TokenValue::Token(gram), cursor))
            },
            ParamType::Time => Self::read_time(cursor),
            ParamType::Expire => Self::read_expire(cursor),
            ParamType::PublicKey => Self::read_public_key(cursor),
            ParamType::Optional(inner_type) => Self::read_optional(&inner_type, cursor, last, abi_version, allow_partial),
            ParamType::Ref(inner_type) => Self::read_ref(&inner_type, cursor, last, abi_version, allow_partial),
        }
    }

    fn read_uint_from_chain(size: usize, cursor: SliceData) -> Result<(BigUint, SliceData)> {
        let (vec, cursor) = get_next_bits_from_chain(cursor, size)?;
        let number = BigUint::from_bytes_be(&vec) >> (vec.len() * 8 - size);
        Ok((number, cursor))
    }

    fn read_int_from_chain(size: usize, cursor: SliceData) -> Result<(BigInt, SliceData)> {
        let (vec, cursor) = get_next_bits_from_chain(cursor, size)?;
        let number = BigInt::from_signed_bytes_be(&vec) >> (vec.len() * 8 - size);
        Ok((number, cursor))
    }

    fn read_uint(size: usize, cursor: SliceData) -> Result<(Self, SliceData)> {
        let (number, cursor) = Self::read_uint_from_chain(size, cursor)?;
        Ok((TokenValue::Uint(Uint { number, size }), cursor))
    }

    fn read_int(size: usize, cursor: SliceData) -> Result<(Self, SliceData)> {
        let (number, cursor) = Self::read_int_from_chain(size, cursor)?;
        Ok((TokenValue::Int(Int { number, size }), cursor))
    }

    fn read_varuint(size: usize, cursor: SliceData) -> Result<(Self, SliceData)> {
        let (len, cursor) = Self::read_uint_from_chain(ParamType::varint_size_len(size), cursor)?;
        let len = len.to_usize().unwrap();
        let (number, cursor) = Self::read_uint_from_chain(len * 8, cursor)?;
        Ok((TokenValue::VarUint(size, number), cursor))
    }

    fn read_varint(size: usize, cursor: SliceData) -> Result<(Self, SliceData)> {
        let (len, cursor) = Self::read_uint_from_chain(ParamType::varint_size_len(size), cursor)?;
        let len = len.to_usize().unwrap();
        let (number, cursor) = Self::read_int_from_chain(len * 8, cursor)?;
        Ok((TokenValue::VarInt(size, number), cursor))
    }

    fn read_tuple(
        tuple_params: &[Param], cursor: SliceData, last: bool, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Self, SliceData)> {
        let mut tokens = Vec::new();
        let mut cursor = cursor;
        for param in tuple_params {
            let last = last && Some(param) == tuple_params.last();
            let (token_value, new_cursor) = TokenValue::read_from(
                &param.kind, cursor, last, abi_version, allow_partial
            )?;
            tokens.push(Token {
                name: param.name.clone(),
                value: token_value,
            });
            cursor = new_cursor;
        }
        Ok((TokenValue::Tuple(tokens), cursor))
    }

    fn check_full_decode(allow_partial: bool, remaining: &SliceData) -> Result<()> {
        if !allow_partial && (remaining.remaining_references() != 0 || remaining.remaining_bits() != 0) {
            fail!(AbiError::IncompleteDeserializationError)
        } else {
            Ok(())
        }
    }

    fn read_array_from_map(
        item_type: &ParamType, mut cursor: SliceData, size: usize, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Vec<Self>, SliceData)> {
        let original = cursor.clone();
        cursor = find_next_bits(cursor, 1)?;
        let map = HashmapE::with_hashmap(32, cursor.get_dictionary()?.reference_opt(0));
        let mut result = vec![];
        for i in 0..size {
            let mut index = BuilderData::new();
            index.append_u32(i as u32)?;
            match map.get(index.into_cell()?.into()) {
                Ok(Some(item_slice)) => {
                    let (token, item_slice) = Self::read_from(
                        item_type, item_slice, true, abi_version, allow_partial
                    )?;
                    Self::check_full_decode(allow_partial, &item_slice)?;
                    result.push(token);
                }
                _ => fail!(AbiError::DeserializationError { msg: "Array doesn't contain item with specified index", cursor: original } )
            }
        }

        Ok((result, cursor))
    }

    fn read_array(
        item_type: &ParamType, mut cursor: SliceData, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 32)?;
        let size = cursor.get_next_u32()?;
        let (result, cursor) = Self::read_array_from_map(
            item_type, cursor, size as usize, abi_version, allow_partial
        )?;

        Ok((TokenValue::Array(item_type.clone(), result), cursor))
    }

    fn read_fixed_array(
        item_type: &ParamType, size: usize, cursor: SliceData, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Self, SliceData)> {
        let (result, cursor) = Self::read_array_from_map(
            item_type, cursor, size, abi_version, allow_partial
        )?;

        Ok((TokenValue::FixedArray(item_type.clone(), result), cursor))
    }

    fn read_cell(mut cursor: SliceData, last: bool, abi_version: &AbiVersion) -> Result<(Cell, SliceData)> {
        let cell = match cursor.remaining_references() {
            1 if (abi_version == &ABI_VERSION_1_0 && cursor.cell().references_count() == BuilderData::references_capacity())
                || (abi_version != &ABI_VERSION_1_0 && !last && cursor.remaining_bits() == 0) => {
                cursor = SliceData::from(cursor.reference(0)?);
                cursor.checked_drain_reference()?
            }
            _ => cursor.checked_drain_reference()?
        };
        Ok((cell.clone(), cursor))
    }

    fn read_hashmap(
        key_type: &ParamType, value_type: &ParamType, mut cursor: SliceData, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 1)?;
        let mut new_map = BTreeMap::new();
        let bit_len = key_type.get_map_key_size()?;
        let hashmap = HashmapE::with_hashmap(bit_len, cursor.get_dictionary()?.reference_opt(0));
        hashmap.iterate_slices(|key, value| {
            let key = Self::read_from(key_type, key, true, abi_version, allow_partial)?.0;
            let key = serde_json::to_value(&key)?.as_str().ok_or(AbiError::InvalidData {
                msg: "Non-ordinary key".to_owned()
            })?.to_owned();
            let value = Self::read_from(value_type, value, true, abi_version, allow_partial)?.0;
            new_map.insert(key, value);
            Ok(true)
        })?;
        Ok((TokenValue::Map(key_type.clone(), value_type.clone(), new_map), cursor))
    }

    fn read_bytes_from_chain(cursor: SliceData, last: bool, abi_version: &AbiVersion) -> Result<(Vec<u8>, SliceData)> {
        let original = cursor.clone();
        let (mut cell, cursor) = Self::read_cell(cursor, last, abi_version)?;

        let mut data = vec![];
        loop {
            if cell.bit_length() % 8 != 0 {
                fail!(AbiError::DeserializationError {
                    msg: "`bytes` cell contains non integer number of bytes",
                    cursor: original
                });
            }
            data.extend_from_slice(cell.data());
            cell = match cell.reference(0) {
                Ok(cell) => cell.clone(),
                Err(_) => break
            };
        }

        Ok((data, cursor))
    }

    fn read_bytes(size: Option<usize>, cursor: SliceData, last: bool, abi_version: &AbiVersion) -> Result<(Self, SliceData)> {
        let original = cursor.clone();
        let (data, cursor) = Self::read_bytes_from_chain(cursor, last, abi_version)?;

        match size {
            Some(size) if size == data.len() => Ok((TokenValue::FixedBytes(data), cursor)),
            Some(_) => fail!(AbiError::DeserializationError {
                msg: "Size of fixed bytes does not correspond to expected size",
                cursor: original
            }),
            None => Ok((TokenValue::Bytes(data), cursor))
        }
    }

    fn read_string(cursor: SliceData, last: bool, abi_version: &AbiVersion) -> Result<(Self, SliceData)> {
        let (data, cursor) = Self::read_bytes_from_chain(cursor, last, abi_version)?;

        let string = String::from_utf8(data)
            .map_err(|err| AbiError::InvalidData {
                msg: format!("Can not deserialize string: {}", err)
            })?;
        Ok((TokenValue::String(string), cursor))
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

    fn read_optional(
        inner_type: &ParamType, cursor: SliceData, last: bool, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Self, SliceData)> {
        let mut cursor = find_next_bits(cursor, 1)?;
        if cursor.get_next_bit()? {
            if inner_type.is_large_optional() {
                let (cell, cursor) = Self::read_cell(cursor, last, abi_version)?;
                let (result, remaining) = Self::read_from(
                    inner_type, cell.into(), true, abi_version, allow_partial
                )?;
                Self::check_full_decode(allow_partial, &remaining)?;
                Ok((TokenValue::Optional(inner_type.clone(), Some(Box::new(result))), cursor))
            } else {
                let (result, cursor) = Self::read_from(
                    inner_type, cursor, last, abi_version, allow_partial
                )?;
                Ok((TokenValue::Optional(inner_type.clone(), Some(Box::new(result))), cursor))
            }
        } else {
            Ok((TokenValue::Optional(inner_type.clone(), None), cursor))
        }
    }

    fn read_ref(
        inner_type: &ParamType, cursor: SliceData, last: bool, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<(Self, SliceData)> {
        let (cell, cursor) = Self::read_cell(cursor, last, abi_version)?;
        let (result, remaining) = Self::read_from(
            inner_type, cell.into(), true, abi_version, allow_partial
        )?;
        Self::check_full_decode(allow_partial, &remaining)?;
        Ok((TokenValue::Ref(Box::new(result)), cursor))
    }

    /// Decodes provided params from SliceData
    pub fn decode_params(
        params: &Vec<Param>, mut cursor: SliceData, abi_version: &AbiVersion, allow_partial: bool
    ) -> Result<Vec<Token>> {
        let mut tokens = vec![];

        for param in params {
            // println!("{:?}", param);
            let last = Some(param) == params.last();
            let (token_value, new_cursor) = Self::read_from(
                &param.kind, cursor, last, abi_version, allow_partial
            )?;

            cursor = new_cursor;
            tokens.push(Token { name: param.name.clone(), value: token_value });
        }

        Self::check_full_decode(allow_partial, &cursor)?;
        
        Ok(tokens)
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
            fail!(AbiError::IncompleteDeserializationError)
        }
        cursor = cursor.reference(0)?.into();
    }
    match cursor.remaining_bits() >= bits  {
        true => Ok(cursor),
        false => fail!(AbiError::DeserializationError { 
            msg: "Not enough remaining bits in the cell", 
            cursor: original
        })
    }
}
