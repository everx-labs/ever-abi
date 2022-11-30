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

use crate::{
    contract::{AbiVersion, ABI_VERSION_1_0, ABI_VERSION_2_2},
    error::AbiError,
    int::{Int, Uint},
    param_type::ParamType,
    token::{Token, TokenValue, Tokenizer},
};

use num_bigint::{BigInt, BigUint, Sign};
use std::collections::BTreeMap;
use ton_block::Serializable;
use ton_types::{error, fail, BuilderData, Cell, HashmapE, IBitstring, Result, SliceData};

pub struct SerializedValue {
    pub data: BuilderData,
    pub max_bits: usize,
    pub max_refs: usize,
}

impl From<BuilderData> for SerializedValue {
    fn from(data: BuilderData) -> Self {
        SerializedValue {
            max_bits: data.bits_used(),
            max_refs: data.references_used(),
            data,
        }
    }
}

impl TokenValue {
    pub fn pack_values_into_chain(
        tokens: &[Token],
        mut cells: Vec<SerializedValue>,
        abi_version: &AbiVersion,
    ) -> Result<BuilderData> {
        for token in tokens {
            cells.append(&mut token.value.write_to_cells(abi_version)?);
        }
        Self::pack_cells_into_chain(cells, abi_version)
    }

    pub fn pack_into_chain(&self, abi_version: &AbiVersion) -> Result<BuilderData> {
        Self::pack_cells_into_chain(self.write_to_cells(abi_version)?, abi_version)
    }

    // first cell is resulting builder
    // every next cell: put data to root
    fn pack_cells_into_chain(
        mut values: Vec<SerializedValue>,
        abi_version: &AbiVersion,
    ) -> Result<BuilderData> {
        values.reverse();
        let mut packed_cells = match values.pop() {
            Some(cell) => vec![cell],
            None => fail!(AbiError::InvalidData {
                msg: "No cells".to_owned()
            }),
        };
        while let Some(value) = values.pop() {
            let builder = packed_cells.last_mut().unwrap();

            let (remaining_bits, remaining_refs) = if abi_version >= &ABI_VERSION_2_2 {
                (
                    BuilderData::bits_capacity() - builder.max_bits,
                    BuilderData::references_capacity() - builder.max_refs,
                )
            } else {
                (builder.data.bits_free(), builder.data.references_free())
            };
            let (value_bits, value_refs) = if abi_version >= &ABI_VERSION_2_2 {
                (value.max_bits, value.max_refs)
            } else {
                (value.data.bits_used(), value.data.references_used())
            };

            if remaining_bits < value_bits || remaining_refs < value_refs {
                // if not enough bits or refs - continue chain
                packed_cells.push(value);
            } else if value_refs > 0 && remaining_refs == value_refs {
                // if refs strictly fit into cell we should decide if we can put them into current
                // cell or to the next cell: if all remaining values can fit into current cell,
                // then use current, if not - continue chain
                let (refs, bits) = Self::get_remaining(&values, abi_version);
                // in ABI v1 last ref is always used for chaining
                if abi_version != &ABI_VERSION_1_0
                    && (refs == 0 && bits + value_bits <= remaining_bits)
                {
                    builder.data.append_builder(&value.data)?;
                    builder.max_bits += value.max_bits;
                    builder.max_refs += value.max_refs;
                } else {
                    packed_cells.push(value);
                }
            } else {
                builder.data.append_builder(&value.data)?;
                builder.max_bits += value.max_bits;
                builder.max_refs += value.max_refs;
            }
        }
        Ok(packed_cells
            .into_iter()
            .rev()
            .reduce(|acc, mut cur| {
                cur.data.append_reference(acc.data);
                cur
            })
            .unwrap()
            .data)
    }

    fn get_remaining(values: &[SerializedValue], abi_version: &AbiVersion) -> (usize, usize) {
        values.iter().fold((0, 0), |(refs, bits), value| {
            if abi_version >= &ABI_VERSION_2_2 {
                (refs + value.max_refs, bits + value.max_bits)
            } else {
                (
                    refs + value.data.references_used(),
                    bits + value.data.bits_used(),
                )
            }
        })
    }

    pub fn write_to_cells(&self, abi_version: &AbiVersion) -> Result<Vec<SerializedValue>> {
        let data = match self {
            TokenValue::Uint(uint) => Self::write_uint(uint),
            TokenValue::Int(int) => Self::write_int(int),
            TokenValue::VarUint(size, uint) => Self::write_varuint(uint, *size),
            TokenValue::VarInt(size, int) => Self::write_varint(int, *size),
            TokenValue::Bool(b) => Self::write_bool(b),
            TokenValue::Tuple(ref tokens) => {
                let mut vec = vec![];
                for token in tokens.iter() {
                    vec.append(&mut token.value.write_to_cells(abi_version)?);
                }
                return Ok(vec);
            }
            TokenValue::Array(param_type, ref tokens) => {
                Self::write_array(param_type, tokens, abi_version)
            }
            TokenValue::FixedArray(param_type, ref tokens) => {
                Self::write_fixed_array(param_type, tokens, abi_version)
            }
            TokenValue::Cell(cell) => Self::write_cell(cell),
            TokenValue::Map(key_type, value_type, value) => {
                Self::write_map(key_type, value_type, value, abi_version)
            }
            TokenValue::Address(address) => Ok(address.write_to_new_cell()?),
            TokenValue::Bytes(ref arr) | TokenValue::FixedBytes(ref arr) => {
                Self::write_bytes(arr, abi_version)
            }
            TokenValue::String(ref string) => Self::write_bytes(string.as_bytes(), abi_version),
            TokenValue::Token(gram) => Ok(gram.write_to_new_cell()?),
            TokenValue::Time(time) => Ok(time.write_to_new_cell()?),
            TokenValue::Expire(expire) => Ok(expire.write_to_new_cell()?),
            TokenValue::PublicKey(key) => Self::write_public_key(key),
            TokenValue::Optional(param_type, value) => Self::write_optional(
                param_type,
                value.as_ref().map(|val| val.as_ref()),
                abi_version,
            ),
            TokenValue::Ref(value) => Self::write_ref(value, abi_version),
        }?;

        let param_type = self.get_param_type();
        Ok(vec![SerializedValue {
            data,
            max_bits: param_type.max_bit_size(),
            max_refs: param_type.max_refs_count(),
        }])
    }

    fn write_int(value: &Int) -> Result<BuilderData> {
        let vec = value.number.to_signed_bytes_be();
        let vec_bits_length = vec.len() * 8;

        let mut builder = BuilderData::new();

        if value.size > vec_bits_length {
            let padding = if value.number.sign() == num_bigint::Sign::Minus {
                0xFFu8
            } else {
                0u8
            };

            let dif = value.size - vec_bits_length;

            let mut vec_padding = Vec::new();
            vec_padding.resize(dif / 8 + 1, padding);

            builder.append_raw(&vec_padding, dif)?;
            builder.append_raw(&vec, value.size - dif)?;
        } else {
            let offset = vec_bits_length - value.size;
            let first_byte = vec[offset / 8] << (offset % 8);

            builder.append_raw(&[first_byte], 8 - offset % 8)?;
            builder.append_raw(&vec[offset / 8 + 1..], vec[offset / 8 + 1..].len() * 8)?;
        };

        Ok(builder)
    }

    fn write_uint(value: &Uint) -> Result<BuilderData> {
        let int = Int {
            number: BigInt::from_biguint(Sign::Plus, value.number.clone()),
            size: value.size,
        };

        Self::write_int(&int)
    }

    fn write_varint(value: &BigInt, size: usize) -> Result<BuilderData> {
        let vec = value.to_signed_bytes_be();

        if vec.len() > size - 1 {
            fail!(AbiError::InvalidData {
                msg: format!("Too long value for varint{}: {}", size, value)
            });
        }

        let mut builder = BuilderData::new();
        let bits = ParamType::varint_size_len(size);
        builder.append_bits(vec.len(), bits as usize)?;
        builder.append_raw(&vec, vec.len() * 8)?;

        Ok(builder)
    }

    fn write_varuint(value: &BigUint, size: usize) -> Result<BuilderData> {
        let big_int = BigInt::from_biguint(Sign::Plus, value.clone());

        Self::write_varint(&big_int, size)
    }

    fn write_bool(value: &bool) -> Result<BuilderData> {
        let mut builder = BuilderData::new();
        builder.append_bit_bool(*value)?;
        Ok(builder)
    }

    fn write_cell(cell: &Cell) -> Result<BuilderData> {
        let mut builder = BuilderData::new();
        builder.append_reference_cell(cell.clone());
        Ok(builder)
    }

    // creates dictionary with indexes of an array items as keys and items as values
    // and prepends dictionary to cell
    fn put_array_into_dictionary(
        param_type: &ParamType,
        array: &[TokenValue],
        abi_version: &AbiVersion,
    ) -> Result<HashmapE> {
        let mut map = HashmapE::with_bit_len(32);

        let value_in_ref = Self::map_value_in_ref(32, param_type.max_bit_size());

        for (i, token) in array.iter().enumerate() {
            let index = SliceData::load_builder((i as u32).write_to_new_cell()?)?;

            let data =
                Self::pack_cells_into_chain(token.write_to_cells(abi_version)?, abi_version)?;

            if value_in_ref {
                map.set_builder(index, &data)?;
            } else {
                map.setref(index, &data.into_cell()?)?;
            }
        }

        Ok(map)
    }

    fn write_array(
        param_type: &ParamType,
        value: &Vec<TokenValue>,
        abi_version: &AbiVersion,
    ) -> Result<BuilderData> {
        let map = Self::put_array_into_dictionary(param_type, value, abi_version)?;

        let mut builder = BuilderData::new();
        builder.append_u32(value.len() as u32)?;

        map.write_to(&mut builder)?;

        Ok(builder)
    }

    fn write_fixed_array(
        param_type: &ParamType,
        value: &[TokenValue],
        abi_version: &AbiVersion,
    ) -> Result<BuilderData> {
        let map = Self::put_array_into_dictionary(param_type, value, abi_version)?;

        map.write_to_new_cell()
    }

    fn write_bytes(data: &[u8], abi_version: &AbiVersion) -> Result<BuilderData> {
        let cell_len = BuilderData::bits_capacity() / 8;
        let mut len = data.len();
        let mut cell_capacity = if abi_version == &ABI_VERSION_1_0 {
            std::cmp::min(cell_len, len)
        } else {
            match len % cell_len {
                0 => cell_len,
                x => x,
            }
        };
        let mut builder = BuilderData::new();
        while len > 0 {
            len -= cell_capacity;
            builder.append_raw(&data[len..len + cell_capacity], cell_capacity * 8)?;
            let mut new_builder = BuilderData::new();
            new_builder.append_reference(builder);
            builder = new_builder;
            cell_capacity = std::cmp::min(cell_len, len);
        }
        // if bytes are empty then we need builder with ref to empty cell
        if builder.references_used() == 0 {
            builder.append_reference(BuilderData::new());
        }
        Ok(builder)
    }

    fn map_value_in_ref(key_len: usize, value_len: usize) -> bool {
        super::MAX_HASH_MAP_INFO_ABOUT_KEY + key_len + value_len <= 1023
    }

    fn write_map(
        key_type: &ParamType,
        value_type: &ParamType,
        value: &BTreeMap<String, TokenValue>,
        abi_version: &AbiVersion,
    ) -> Result<BuilderData> {
        let key_len = key_type.get_map_key_size()?;
        let value_len = value_type.max_bit_size();
        let value_in_ref = Self::map_value_in_ref(key_len, value_len);

        let mut hashmap = HashmapE::with_bit_len(key_len);

        for (key, value) in value.iter() {
            let key = Tokenizer::tokenize_parameter(key_type, &key.as_str().into(), "map key")?;

            let mut key_vec = key.write_to_cells(abi_version)?;
            if key_vec.len() != 1 {
                fail!(AbiError::InvalidData {
                    msg: "Map key must be 1-cell length".to_owned()
                })
            };
            if &ParamType::Address == key_type
                && key_vec[0].data.length_in_bits() != super::STD_ADDRESS_BIT_LENGTH
            {
                fail!(AbiError::InvalidData {
                    msg: "Only std non-anycast address can be used as map key".to_owned()
                })
            }

            let data =
                Self::pack_cells_into_chain(value.write_to_cells(abi_version)?, abi_version)?;

            let slice_key = SliceData::load_builder(key_vec.pop().unwrap().data)?;
            if value_in_ref {
                hashmap.set_builder(slice_key, &data)?;
            } else {
                hashmap.setref(slice_key, &data.into_cell()?)?;
            }
        }

        let mut builder = BuilderData::new();
        hashmap.write_to(&mut builder)?;

        Ok(builder)
    }

    fn write_public_key(data: &Option<ed25519_dalek::PublicKey>) -> Result<BuilderData> {
        let mut builder = BuilderData::new();
        if let Some(key) = data {
            builder.append_bit_one()?;
            let bytes = &key.to_bytes()[..];
            let length = bytes.len() * 8;
            builder.append_raw(bytes, length)?;
        } else {
            builder.append_bit_zero()?;
        }
        Ok(builder)
    }

    fn write_optional(
        param_type: &ParamType,
        value: Option<&TokenValue>,
        abi_version: &AbiVersion,
    ) -> Result<BuilderData> {
        if let Some(value) = value {
            if param_type.is_large_optional() {
                let value = value.pack_into_chain(abi_version)?;
                let mut builder = BuilderData::new();
                builder.append_bit_one()?;
                builder.checked_append_reference(value.into_cell()?)?;
                Ok(builder)
            } else {
                let mut builder = value.pack_into_chain(abi_version)?;
                builder.prepend_raw(&[0x80], 1)?;
                Ok(builder)
            }
        } else {
            Ok(BuilderData::with_raw(vec![0x00], 1)?)
        }
    }

    fn write_ref(value: &TokenValue, abi_version: &AbiVersion) -> Result<BuilderData> {
        let value = value.pack_into_chain(abi_version)?;
        let mut builder = BuilderData::new();
        builder.checked_append_reference(value.into_cell()?)?;
        Ok(builder)
    }
}

#[test]
fn test_pack_cells() {
    let cells = vec![
        BuilderData::with_bitstring(vec![1, 2, 0x80])
            .unwrap()
            .into(),
        BuilderData::with_bitstring(vec![3, 4, 0x80])
            .unwrap()
            .into(),
    ];
    let builder = BuilderData::with_bitstring(vec![1, 2, 3, 4, 0x80]).unwrap();
    assert_eq!(
        TokenValue::pack_cells_into_chain(cells, &ABI_VERSION_1_0).unwrap(),
        builder
    );

    let cells = vec![
        BuilderData::with_raw(vec![0x55; 100], 100 * 8)
            .unwrap()
            .into(),
        BuilderData::with_raw(vec![0x55; 127], 127 * 8)
            .unwrap()
            .into(),
        BuilderData::with_raw(vec![0x55; 127], 127 * 8)
            .unwrap()
            .into(),
    ];

    let builder = BuilderData::with_raw(vec![0x55; 127], 127 * 8).unwrap();
    let builder = BuilderData::with_raw_and_refs(
        vec![0x55; 127],
        127 * 8,
        vec![builder.into_cell().unwrap()],
    )
    .unwrap();
    let builder = BuilderData::with_raw_and_refs(
        vec![0x55; 100],
        100 * 8,
        vec![builder.into_cell().unwrap()],
    )
    .unwrap();
    let tree = TokenValue::pack_cells_into_chain(cells, &ABI_VERSION_1_0).unwrap();
    assert_eq!(tree, builder);
}
