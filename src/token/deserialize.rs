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

use crate::{
    contract::{AbiVersion, ABI_VERSION_1_0, ABI_VERSION_2_2},
    error::AbiError,
    int::{Int, Uint},
    param::Param,
    param_type::ParamType,
    token::{Token, TokenValue},
};

use num_bigint::{BigInt, BigUint};
use num_traits::ToPrimitive;
use serde_json;
use std::{collections::BTreeMap, convert::TryInto};
use ton_block::{types::Grams, MsgAddress};
use ton_types::{
    error, fail, BuilderData, Cell, HashmapE, HashmapType, IBitstring, Result, SliceData,
};

#[derive(Clone, Debug, Default)]
pub struct Cursor {
    pub used_bits: usize,
    pub used_refs: usize,
    pub slice: SliceData,
}

impl From<SliceData> for Cursor {
    fn from(slice: SliceData) -> Self {
        Self { used_bits: 0, used_refs: 0, slice }
    }
}

impl TokenValue {
    /// Deserializes value from `SliceData` to `TokenValue`
    fn read_from(
        param_type: &ParamType,
        mut cursor: Cursor,
        last: bool,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Self, Cursor)> {
        let slice = cursor.slice.clone();
        let (value, slice) = match dbg!(param_type) {
            ParamType::Uint(size) => Self::read_uint(*size, slice),
            ParamType::Int(size) => Self::read_int(*size, slice),
            ParamType::VarUint(size) => Self::read_varuint(*size, slice),
            ParamType::VarInt(size) => Self::read_varint(*size, slice),
            ParamType::Bool => {
                let mut slice = find_next_bits(slice, 1)?;
                Ok((TokenValue::Bool(slice.get_next_bit()?), slice))
            }
            ParamType::Tuple(tuple_params) => {
                return Self::read_tuple(tuple_params, cursor, last, abi_version, allow_partial);
            }
            ParamType::Array(item_type) => {
                Self::read_array(&item_type, slice, abi_version, allow_partial)
            }
            ParamType::FixedArray(item_type, size) => {
                Self::read_fixed_array(&item_type, *size, slice, abi_version, allow_partial)
            }
            ParamType::Cell => Self::read_cell(slice, last, abi_version)
                .map(|(cell, slice)| (TokenValue::Cell(cell), slice)),
            ParamType::Map(key_type, value_type) => {
                Self::read_hashmap(key_type, value_type, slice, abi_version, allow_partial)
            }
            ParamType::Address => {
                let mut slice = find_next_bits(slice, 1)?;
                let address =
                    <MsgAddress as ton_block::Deserializable>::construct_from(&mut slice)?;
                Ok((TokenValue::Address(address), slice))
            }
            ParamType::Bytes => Self::read_bytes(None, slice, last, abi_version),
            ParamType::FixedBytes(size) => Self::read_bytes(Some(*size), slice, last, abi_version),
            ParamType::String => Self::read_string(slice, last, abi_version),
            ParamType::Token => {
                let mut slice = find_next_bits(slice, 1)?;
                let gram = <Grams as ton_block::Deserializable>::construct_from(&mut slice)?;
                Ok((TokenValue::Token(gram), slice))
            }
            ParamType::Time => Self::read_time(slice),
            ParamType::Expire => Self::read_expire(slice),
            ParamType::PublicKey => Self::read_public_key(slice),
            ParamType::Optional(inner_type) => {
                Self::read_optional(&inner_type, slice, last, abi_version, allow_partial)
            }
            ParamType::Ref(inner_type) => {
                Self::read_ref(&inner_type, slice, last, abi_version, allow_partial)
            }
        }?;

        if last {
            Self::check_full_decode(allow_partial, &slice)?;
        }

        cursor = Self::check_layout(param_type, cursor, &slice, abi_version, last)?;
        cursor.slice = slice;

        Ok((value, cursor))
    }

    fn check_layout(
        param_type: &ParamType,
        original_cursor: Cursor,
        new_slice: &SliceData,
        abi_version: &AbiVersion,
        last: bool,
    ) -> Result<Cursor> {
        let mut cursor = original_cursor;
        let new_cell = new_slice.cell_opt();
        let orig_cell = cursor.slice.cell_opt();
        if abi_version >= &ABI_VERSION_2_2 {
            let param_max_bits = Self::max_bit_size(param_type);
            let param_max_refs = Self::max_refs_count(param_type);
            if new_cell != orig_cell {
                if  cursor.used_bits + param_max_bits <= BuilderData::bits_capacity() && 
                    (last && cursor.used_refs + param_max_refs <= BuilderData::references_capacity() ||
                    !last && cursor.used_refs + param_max_refs <= BuilderData::references_capacity() - 1)
                {
                    fail!(AbiError::WrongDataLayout);
                }
                cursor.used_bits = param_max_bits;
                cursor.used_refs = param_max_refs;
            } else {
                cursor.used_bits += param_max_bits;
                cursor.used_refs += param_max_refs;
                if  cursor.used_bits > BuilderData::bits_capacity() ||
                    cursor.used_refs > BuilderData::references_capacity()
                {
                    fail!(AbiError::WrongDataLayout);
                }
            }
        } else {
            if new_cell != orig_cell {
                // following error will never appear because SliceData::cell_opt function returns
                // None only if slice contains just data without refs. And if there is no refs then
                // cursor cell can not change
                let orig_cell = orig_cell
                    .ok_or_else(|| AbiError::DeserializationError { 
                        msg: "No original cell in layout check", cursor: cursor.slice.clone()
                    })?;

                let param_bits = new_slice.pos();
                let param_refs = new_slice.get_references().start;

                if  param_bits <= BuilderData::bits_capacity() - orig_cell.bit_length() && 
                    (last && param_refs + orig_cell.references_count() <= BuilderData::references_capacity() ||
                    (!last || abi_version == &ABI_VERSION_1_0) && param_refs + orig_cell.references_count() <= BuilderData::references_capacity() - 1)
                {
                    fail!(AbiError::WrongDataLayout);
                }
            }
        }

        Ok(cursor)
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
        let (len, cursor) = Self::read_uint_from_chain(TokenValue::varint_size_len(size), cursor)?;
        let len = len.to_usize().unwrap();
        if len == 0 {
            Ok((TokenValue::VarUint(size, 0u32.into()), cursor))
        } else {
            let (number, cursor) = Self::read_uint_from_chain(len * 8, cursor)?;
            Ok((TokenValue::VarUint(size, number), cursor))
        }
    }

    fn read_varint(size: usize, cursor: SliceData) -> Result<(Self, SliceData)> {
        let (len, cursor) = Self::read_uint_from_chain(TokenValue::varint_size_len(size), cursor)?;
        let len = len.to_usize().unwrap();
        if len == 0 {
            Ok((TokenValue::VarInt(size, 0.into()), cursor))
        } else {
            let (number, cursor) = Self::read_int_from_chain(len * 8, cursor)?;
            Ok((TokenValue::VarInt(size, number), cursor))
        }
    }

    fn read_tuple(
        tuple_params: &[Param],
        cursor: Cursor,
        last: bool,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Self, Cursor)> {
        let (tokens, cursor) = Self::decode_params_with_cursor(
            tuple_params, cursor, abi_version, allow_partial || !last
        )?;
        Ok((TokenValue::Tuple(tokens), cursor))
    }

    fn check_full_decode(allow_partial: bool, remaining: &SliceData) -> Result<()> {
        if !allow_partial
            && (remaining.remaining_references() != 0 || remaining.remaining_bits() != 0)
        {
            fail!(AbiError::IncompleteDeserializationError)
        } else {
            Ok(())
        }
    }

    fn read_array_from_map(
        item_type: &ParamType,
        mut cursor: SliceData,
        size: usize,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Vec<Self>, SliceData)> {
        let value_len = Self::max_bit_size(item_type);
        let value_in_ref = Self::map_value_in_ref(32, value_len);

        let original = cursor.clone();
        cursor = find_next_bits(cursor, 1)?;
        let map = HashmapE::with_hashmap(32, cursor.get_dictionary()?.reference_opt(0));
        if map.count(size + 1)? != size {
            fail!(AbiError::DeserializationError {
                msg: "Array contains more items then declared",
                cursor: original
            })
        }
        let mut result = vec![];
        for i in 0..size {
            let mut index = BuilderData::new();
            index.append_u32(i as u32)?;
            match map.get(SliceData::load_builder(index)?) {
                Ok(Some(mut item_slice)) => {
                    if value_in_ref {
                        item_slice = SliceData::load_cell(item_slice.checked_drain_reference()?)?;
                    }
                    let (token, _) =
                        Self::read_from(item_type, item_slice.into(), true, abi_version, allow_partial)?;
                    result.push(token);
                }
                _ => fail!(AbiError::DeserializationError {
                    msg: "Array doesn't contain item with specified index",
                    cursor: original
                }),
            }
        }

        Ok((result, cursor))
    }

    fn read_array(
        item_type: &ParamType,
        mut cursor: SliceData,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Self, SliceData)> {
        cursor = find_next_bits(cursor, 32)?;
        let size = cursor.get_next_u32()?;
        let (result, cursor) = Self::read_array_from_map(
            item_type,
            cursor,
            size as usize,
            abi_version,
            allow_partial,
        )?;

        Ok((TokenValue::Array(item_type.clone(), result), cursor))
    }

    fn read_fixed_array(
        item_type: &ParamType,
        size: usize,
        cursor: SliceData,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Self, SliceData)> {
        let (result, cursor) =
            Self::read_array_from_map(item_type, cursor, size, abi_version, allow_partial)?;

        Ok((TokenValue::FixedArray(item_type.clone(), result), cursor))
    }

    fn read_cell(
        mut cursor: SliceData,
        last: bool,
        abi_version: &AbiVersion,
    ) -> Result<(Cell, SliceData)> {
        let cell = match cursor.remaining_references() {
            1 if (abi_version == &ABI_VERSION_1_0
                && cursor.cell().references_count() == BuilderData::references_capacity())
                || (abi_version != &ABI_VERSION_1_0 && !last && cursor.remaining_bits() == 0) =>
            {
                cursor = SliceData::load_cell(cursor.reference(0)?)?;
                cursor.checked_drain_reference()?
            }
            _ => cursor.checked_drain_reference()?,
        };
        Ok((cell.clone(), cursor))
    }

    fn read_hashmap(
        key_type: &ParamType,
        value_type: &ParamType,
        mut cursor: SliceData,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Self, SliceData)> {
        let bit_len = TokenValue::get_map_key_size(key_type)?;
        let value_len = Self::max_bit_size(value_type);
        let value_in_ref = Self::map_value_in_ref(bit_len, value_len);

        cursor = find_next_bits(cursor, 1)?;
        let mut new_map = BTreeMap::new();
        let hashmap = HashmapE::with_hashmap(bit_len, cursor.get_dictionary()?.reference_opt(0));
        hashmap.iterate_slices(|key, mut value| {
            let key = Self::read_from(key_type, key.into(), true, abi_version, allow_partial)?.0;
            let key = serde_json::to_value(&key)?
                .as_str()
                .ok_or(AbiError::InvalidData {
                    msg: "Non-ordinary key".to_owned(),
                })?
                .to_owned();
            if value_in_ref {
                value = SliceData::load_cell(value.checked_drain_reference()?)?;
            }
            let value = Self::read_from(value_type, value.into(), true, abi_version, allow_partial)?.0;
            new_map.insert(key, value);
            Ok(true)
        })?;
        Ok((
            TokenValue::Map(key_type.clone(), value_type.clone(), new_map),
            cursor,
        ))
    }

    fn read_bytes_from_chain(
        cursor: SliceData,
        last: bool,
        abi_version: &AbiVersion,
    ) -> Result<(Vec<u8>, SliceData)> {
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
                Err(_) => break,
            };
        }

        Ok((data, cursor))
    }

    fn read_bytes(
        size: Option<usize>,
        cursor: SliceData,
        last: bool,
        abi_version: &AbiVersion,
    ) -> Result<(Self, SliceData)> {
        let original = cursor.clone();
        let (data, cursor) = Self::read_bytes_from_chain(cursor, last, abi_version)?;

        match size {
            Some(size) if size == data.len() => Ok((TokenValue::FixedBytes(data), cursor)),
            Some(_) => fail!(AbiError::DeserializationError {
                msg: "Size of fixed bytes does not correspond to expected size",
                cursor: original
            }),
            None => Ok((TokenValue::Bytes(data), cursor)),
        }
    }

    fn read_string(
        cursor: SliceData,
        last: bool,
        abi_version: &AbiVersion,
    ) -> Result<(Self, SliceData)> {
        let (data, cursor) = Self::read_bytes_from_chain(cursor, last, abi_version)?;

        let string = String::from_utf8(data).map_err(|err| AbiError::InvalidData {
            msg: format!("Can not deserialize string: {}", err),
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
            let bytes = vec
                .try_into()
                .map_err(|_| error!("Invalid public key length"))?;
            Ok((TokenValue::PublicKey(Some(bytes)), cursor))
        } else {
            Ok((TokenValue::PublicKey(None), cursor))
        }
    }

    fn read_optional(
        inner_type: &ParamType,
        cursor: SliceData,
        last: bool,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Self, SliceData)> {
        let mut cursor = find_next_bits(cursor, 1)?;
        if cursor.get_next_bit()? {
            if Self::is_large_optional(inner_type) {
                let cell = cursor.checked_drain_reference()?;
                let (result, _) = Self::read_from(
                    inner_type,
                    SliceData::load_cell(cell)?.into(),
                    true,
                    abi_version,
                    allow_partial,
                )?;
                Ok((
                    TokenValue::Optional(inner_type.clone(), Some(Box::new(result))),
                    cursor,
                ))
            } else {
                let (result, cursor) =
                    Self::read_from(inner_type, cursor.into(), last, abi_version, allow_partial)?;
                Ok((
                    TokenValue::Optional(inner_type.clone(), Some(Box::new(result))),
                    cursor.slice,
                ))
            }
        } else {
            Ok((TokenValue::Optional(inner_type.clone(), None), cursor))
        }
    }

    fn read_ref(
        inner_type: &ParamType,
        cursor: SliceData,
        last: bool,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Self, SliceData)> {
        let (cell, cursor) = Self::read_cell(cursor, last, abi_version)?;
        let (result, _) = Self::read_from(
            inner_type,
            SliceData::load_cell(cell)?.into(),
            true,
            abi_version,
            allow_partial,
        )?;
        Ok((TokenValue::Ref(Box::new(result)), cursor))
    }

    /// Decodes provided params from SliceData
    pub fn decode_params(
        params: &[Param],
        cursor: SliceData,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<Vec<Token>> {
        Self::decode_params_with_cursor(params, cursor.into(), abi_version, allow_partial)
            .map(|(tokens, _)| tokens)
    }

    pub fn decode_params_with_cursor(
        params: &[Param],
        mut cursor: Cursor,
        abi_version: &AbiVersion,
        allow_partial: bool,
    ) -> Result<(Vec<Token>, Cursor)> {
        let mut tokens = vec![];

        for param in params {
            // println!("{:?}", param);
            let last = Some(param) == params.last();
            let (token_value, new_cursor) =
                Self::read_from(&param.kind, cursor, last, abi_version, allow_partial)?;

            cursor = new_cursor;
            tokens.push(Token {
                name: param.name.clone(),
                value: token_value,
            });
        }

        Ok((tokens, cursor))
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
        cursor = SliceData::load_cell(cursor.reference(0)?)?;
    }
    match cursor.remaining_bits() >= bits {
        true => Ok(cursor),
        false => fail!(AbiError::DeserializationError {
            msg: "Not enough remaining bits in the cell",
            cursor: original
        }),
    }
}
