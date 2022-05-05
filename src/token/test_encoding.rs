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

use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::str::FromStr;
use num_bigint::{BigInt, BigUint};

use ton_types::{AccountId, BuilderData, Cell, IBitstring, Result, SliceData};
use ton_types::dictionary::{HashmapE, HashmapType};
use ton_block::{AnycastInfo, Grams, MsgAddress, Serializable};

use crate::contract::{ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_1, ABI_VERSION_2_2, AbiVersion, MAX_SUPPORTED_VERSION};

use {Int, Param, ParamType, Token, TokenValue, Uint};

fn put_array_into_map<T: Serializable>(array: &[T]) -> HashmapE {
    let mut map = HashmapE::with_bit_len(32);

    for i in 0..array.len() {
        let index = (i as u32).serialize().unwrap();
        let data = array[i].write_to_new_cell().unwrap();
        map.set_builder(index.into(), &data).unwrap();
    }

    map
}

fn add_array_as_map<T: Serializable>(builder: &mut BuilderData, array: &[T], fixed: bool) {
    if !fixed {
        builder.append_u32(array.len() as u32).unwrap();
    }

    let map = put_array_into_map(array);

    match map.data() {
        Some(cell) => {
            builder.append_bit_one().unwrap();
            builder.append_reference_cell(cell.clone());
        }
        None => { builder.append_bit_zero().unwrap(); }
    }
}

fn test_parameters_set(
    inputs: &[Token],
    params: Option<&[Param]>,
    params_tree: BuilderData,
    versions: &[AbiVersion],
) {
    for version in versions {
        let mut prefix = BuilderData::new();
        prefix.append_reference(BuilderData::new());
        prefix.append_u32(0).unwrap();

        // tree check
        let test_tree = TokenValue::pack_values_into_chain(inputs, vec![prefix.into()], version).unwrap();

        println!("{:#.2}", test_tree.clone().into_cell().unwrap());
        println!("{:#.2}", params_tree.clone().into_cell().unwrap());
        assert_eq!(test_tree, params_tree);

        // check decoding

        let params: Vec<Param> = if let Some(params) = params {
            params.to_vec()
        } else {
            params_from_tokens(inputs)
        };

        let mut slice = SliceData::from(test_tree.into_cell().unwrap());
        slice.checked_drain_reference().unwrap();
        slice.get_next_u32().unwrap();

        let decoded_tokens = TokenValue::decode_params(
            &params, slice, &version.clone().into(), false
        ).unwrap();
        assert_eq!(decoded_tokens, inputs);
    }
}

fn params_from_tokens(tokens: &[Token]) -> Vec<Param> {
     tokens.iter().map(|ref token| token.get_param()).collect()
}

fn tokens_from_values(values: Vec<TokenValue>) -> Vec<Token> {
    let param_names = vec![
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
        "s", "t", "u", "v", "w", "x", "y", "z",
    ];

    values
        .into_iter()
        .zip(param_names)
        .map(|(value, name)| Token {
            name: name.to_owned(),
            value: value,
        })
        .collect()
}

#[test]
fn test_one_input_and_output() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_u128(1123).unwrap();

    let values = vec![TokenValue::Uint(Uint {
        number: BigUint::from(1123u128),
        size: 128,
    })];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_with_grams() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    let grams = Grams::from(173742);
    grams.write_to(&mut builder).unwrap();

    let values = vec![TokenValue::Token(grams)];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_with_address() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_reference(BuilderData::with_bitstring(vec![1, 2, 3, 0x80]).unwrap());

    let anycast = AnycastInfo::with_rewrite_pfx(SliceData::new(vec![0x77, 0x78, 0x79, 0x80])).unwrap();
    let addresses = vec![
        MsgAddress::AddrNone,
        MsgAddress::with_extern(SliceData::new(vec![0x55, 0x80])).unwrap(),
        MsgAddress::with_standart(Some(anycast.clone()), -1, AccountId::from([0x11; 32])).unwrap(),
        MsgAddress::with_standart(Some(anycast.clone()), -1, AccountId::from([0x11; 32])).unwrap(),
        MsgAddress::with_variant(Some(anycast.clone()), -128, SliceData::new(vec![0x66, 0x67, 0x68, 0x69, 0x80])).unwrap(),
        MsgAddress::with_standart(Some(anycast.clone()), -1, AccountId::from([0x11; 32])).unwrap(),
    ];
    let mut builder_v2_2 = builder.clone();
    let mut builders: Vec<BuilderData> = addresses.iter().map(|address| address.write_to_new_cell().unwrap()).collect();
    builders.reverse();
    builder_v2_2.append_builder(&builders.pop().unwrap()).unwrap();
    builders.push(builder_v2_2);
    let builder_v2_2 = builders.into_iter().reduce(
        |acc, mut cur| {
            cur.append_reference(acc);
            cur
        }).unwrap();

    addresses.iter().take(5).for_each(|address| address.write_to(&mut builder).unwrap());
    builder.append_reference(addresses.last().unwrap().write_to_new_cell().unwrap());

    let mut values = vec![TokenValue::Cell(BuilderData::with_bitstring(vec![1, 2, 3, 0x80]).unwrap().into_cell().unwrap())];
    addresses.iter().for_each(|address| {
        values.push(TokenValue::Address(address.clone()));
    });

    test_parameters_set(
        &tokens_from_values(values.clone()),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0],
    );

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder_v2_2,
        &[ABI_VERSION_2_2],
    );
}

#[test]
fn test_one_input_and_output_by_data() {
    // test prefix with one ref and u32
    let mut expected_tree = BuilderData::with_bitstring(vec![
        0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x75, 0x0C, 0xE4, 0x7B, 0xAC, 0x80,
    ]).unwrap();
    expected_tree.append_reference(BuilderData::new());

    let values = vec![TokenValue::Int(Int {
        number: BigInt::from(-596784153684i64),
        size: 64,
    })];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        expected_tree,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_empty_params() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    test_parameters_set(
        &[],
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_two_params() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_bit_one().unwrap();
    builder.append_i32(9434567).unwrap();

    let values = vec![
        TokenValue::Bool(true),
        TokenValue::Int(Int {
            number: BigInt::from(9434567),
            size: 32,
        }),
    ];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_five_refs_v1() {
    let bytes = vec![0x55; 300]; // 300 = 127 + 127 + 46
    let mut builder = BuilderData::with_raw(vec![0x55; 127], 127 * 8).unwrap();
    builder.append_reference(BuilderData::with_raw(vec![0x55; 127], 127 * 8).unwrap());
    let mut bytes_builder = BuilderData::with_raw(vec![0x55; 46], 46 * 8).unwrap();
    bytes_builder.append_reference(builder);

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_bit_one().unwrap();
    builder.append_reference(bytes_builder.clone());
    builder.append_reference(bytes_builder.clone());

    let mut new_builder = BuilderData::new();
    new_builder.append_i32(9434567).unwrap();
    new_builder.append_reference(BuilderData::new());
    new_builder.append_reference(bytes_builder.clone());
    builder.append_reference(new_builder);

    let values = vec![
        TokenValue::Bool(true),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Bytes(vec![]),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Int(Int::new(9434567, 32)),
    ];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0],
    );
}


#[test]
fn test_five_refs_v2() {
    let bytes = vec![0x55; 300]; // 300 = 127 + 127 + 46
    let mut builder = BuilderData::with_raw(vec![0x55; 127], 127 * 8).unwrap();
    builder.append_reference(BuilderData::with_raw(vec![0x55; 46], 46 * 8).unwrap());
    let mut bytes_builder = BuilderData::with_raw(vec![0x55; 127], 127 * 8).unwrap();
    bytes_builder.append_reference(builder);

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_bit_one().unwrap();
    builder.append_reference(bytes_builder.clone());
    builder.append_reference(bytes_builder.clone());

    let mut new_builder = BuilderData::new();
    new_builder.append_i32(9434567).unwrap();
    new_builder.append_reference(BuilderData::new());
    new_builder.append_reference(bytes_builder.clone());
    builder.append_reference(new_builder);

    let values = vec![
        TokenValue::Bool(true),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Bytes(vec![]),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Int(Int::new(9434567, 32)),
    ];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_nested_tuples_with_all_simples() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());


    builder.append_bit_zero().unwrap();
    builder.append_i8(-15 as i8).unwrap();
    builder.append_i16(9845 as i16).unwrap();
    builder.append_i32(-1 as i32).unwrap();
    builder.append_i64(12345678 as i64).unwrap();
    builder.append_i128(-12345678 as i128).unwrap();
    builder.append_u8(255 as u8).unwrap();
    builder.append_u16(0 as u16).unwrap();
    builder.append_u32(256 as u32).unwrap();
    builder.append_u64(123 as u64).unwrap();
    builder.append_u128(1234567890 as u128).unwrap();

    let values = vec![
        TokenValue::Bool(false),
        TokenValue::Tuple(tokens_from_values(vec![
            TokenValue::Int(Int::new(-15, 8)),
            TokenValue::Int(Int::new(9845, 16)),
            TokenValue::Tuple(tokens_from_values(vec![
                TokenValue::Int(Int::new(-1, 32)),
                TokenValue::Int(Int::new(12345678, 64)),
                TokenValue::Int(Int::new(-12345678, 128)),
            ])),
        ])),
        TokenValue::Tuple(tokens_from_values(vec![
            TokenValue::Uint(Uint::new(255, 8)),
            TokenValue::Uint(Uint::new(0, 16)),
            TokenValue::Tuple(tokens_from_values(vec![
                TokenValue::Uint(Uint::new(256, 32)),
                TokenValue::Uint(Uint::new(123, 64)),
                TokenValue::Uint(Uint::new(1234567890, 128)),
            ])),
        ])),
    ];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_static_array_of_ints() {
    let input_array: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    add_array_as_map(&mut builder, &input_array, true);

    let values = vec![TokenValue::FixedArray(
        ParamType::Uint(32),
        input_array
            .iter()
            .map(|i| TokenValue::Uint(Uint::new(i.to_owned() as u128, 32)))
            .collect(),
    )];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_empty_dynamic_array() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    add_array_as_map(&mut builder, &Vec::<u16>::new(), false);

    let values = vec![TokenValue::Array(ParamType::Uint(16), vec![])];

    let params = vec![Param {
        name: "a".to_owned(),
        kind: ParamType::Array(Box::new(ParamType::Uint(16))),
    }];

    test_parameters_set(
        &tokens_from_values(values),
        Some(&params),
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_dynamic_array_of_ints() {
    let input_array: Vec<u16> = vec![1, 2, 3, 4, 5, 6, 7, 8];

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    add_array_as_map(&mut builder, &input_array, false);

    let values = vec![TokenValue::Array(
        ParamType::Uint(16),
        input_array
            .iter()
            .map(|i| TokenValue::Uint(Uint::new(i.to_owned() as u128, 16)))
            .collect(),
    )];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

struct TupleDwordBool(u32, bool);

impl Serializable for TupleDwordBool {
    fn write_to(&self, cell: &mut BuilderData) -> Result<()> {
        self.0.write_to(cell)?;
        self.1.write_to(cell)?;
        Ok(())
    }
}

impl From<&(u32, bool)> for TupleDwordBool {
    fn from(a: &(u32, bool)) -> Self {
        TupleDwordBool(a.0, a.1)
    }
}

#[test]
fn test_dynamic_array_of_tuples() {
    let input_array: Vec<(u32, bool)> =
        vec![(1, true), (2, false), (3, true), (4, false), (5, true)];

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    let bitstring_array: Vec<TupleDwordBool> = input_array
        .iter()
        .map(|a| TupleDwordBool::from(a))
        .collect();

    add_array_as_map(&mut builder, &bitstring_array, false);

    let expected_tree = builder.into();
    
    let values = vec![TokenValue::Array(
        ParamType::Tuple(vec![
            Param::new("a", ParamType::Uint(32)),
            Param::new("b", ParamType::Bool),
        ]),
        input_array
            .iter()
            .map(|i| {
                TokenValue::Tuple(tokens_from_values(vec![
                    TokenValue::Uint(Uint::new(i.0 as u128, 32)),
                    TokenValue::Bool(i.1),
                ]))
            })
            .collect(),
    )];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        expected_tree,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_tuples_with_combined_types() {
    let input_array1: Vec<(u32, bool)> = vec![(1, true), (2, false), (3, true), (4, false)];

    let bitstring_array1: Vec<TupleDwordBool> = input_array1
        .iter()
        .map(|a| TupleDwordBool::from(a))
        .collect();

    let mut input_array2 = Vec::<u64>::new();
    for i in 0..73 {
        input_array2.push(i * i);
    }

    // test prefix with one ref and u32
    let mut chain_builder = BuilderData::new();
    chain_builder.append_u32(0).unwrap();
    chain_builder.append_reference(BuilderData::new());

    // u8
    chain_builder.append_u8(18).unwrap();


    // Vec<(u32, bool)>
    add_array_as_map(&mut chain_builder, &bitstring_array1, false);

    // i16
    chain_builder.append_i16(-290 as i16).unwrap();

    // input_array2
    add_array_as_map(&mut chain_builder, &input_array2, false);

    let mut map = HashmapE::with_bit_len(32);

    // [Vec<i64>; 5]
    for i in 0..5u32 {
        let mut builder = BuilderData::new();
        add_array_as_map(&mut builder, &input_array2, false);
        map.set_builder(i.serialize().unwrap().into(), &builder).unwrap();
    }

    let mut chain_builder_v2 = chain_builder.clone();
    chain_builder_v2.append_bit_one().unwrap();
    chain_builder_v2.append_reference(BuilderData::from(map.data().unwrap().clone()));

    let mut second_builder = BuilderData::new();
    second_builder.append_bit_one().unwrap();
    second_builder.append_reference(BuilderData::from(map.data().unwrap().clone()));

    chain_builder.append_reference(second_builder);

    let array1_token_value = TokenValue::Array(
        ParamType::Tuple(vec![
            Param::new("a", ParamType::Uint(32)),
            Param::new("b", ParamType::Bool),
        ]),
        input_array1
            .iter()
            .map(|i| {
                TokenValue::Tuple(tokens_from_values(vec![
                    TokenValue::Uint(Uint::new(i.0 as u128, 32)),
                    TokenValue::Bool(i.1),
                ]))
            })
            .collect(),
    );

    let array2_token_value = TokenValue::Array(
        ParamType::Int(64),
        input_array2
            .iter()
            .map(|i| TokenValue::Int(Int::new(*i as i128, 64)))
            .collect(),
    );

    let array3_token_value = TokenValue::FixedArray(
        ParamType::Array(Box::new(ParamType::Int(64))),
        vec![
            array2_token_value.clone(),
            array2_token_value.clone(),
            array2_token_value.clone(),
            array2_token_value.clone(),
            array2_token_value.clone(),
    ]);

    let values = vec![
        TokenValue::Uint(Uint::new(18, 8)),
        TokenValue::Tuple(tokens_from_values(vec![
            array1_token_value,
            TokenValue::Int(Int::new(-290, 16)),
        ])),
        TokenValue::Tuple(tokens_from_values(vec![
            array2_token_value,
            array3_token_value,
        ])),
    ];

    test_parameters_set(
        &tokens_from_values(values.clone()),
        None,
        chain_builder,
        &[ABI_VERSION_1_0],
    );

    test_parameters_set(
        &tokens_from_values(values),
        None,
        chain_builder_v2,
        &[ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_four_refs_and_four_int256() {
    let bytes = vec![0x55; 32];
    let bytes_builder = BuilderData::with_raw(bytes.clone(), bytes.len() * 8).unwrap();

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_reference(bytes_builder.clone());
    builder.append_reference(bytes_builder.clone());

    let mut second_builder = BuilderData::new();
    second_builder.append_reference(bytes_builder.clone());
    second_builder.append_builder(&bytes_builder).unwrap();
    second_builder.append_builder(&bytes_builder).unwrap();
    second_builder.append_builder(&bytes_builder).unwrap();

    let mut third_builder = BuilderData::new();
    third_builder.append_builder(&bytes_builder).unwrap();

    second_builder.append_reference(third_builder);
    builder.append_reference(second_builder);

    let values = vec![
        TokenValue::Cell(bytes_builder.clone().into_cell().unwrap()),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Cell(bytes_builder.into_cell().unwrap()),
        TokenValue::Uint(Uint{ number: BigUint::from_bytes_be(&bytes), size: 256 }),
        TokenValue::Uint(Uint{ number: BigUint::from_bytes_be(&bytes), size: 256 }),
        TokenValue::Uint(Uint{ number: BigUint::from_bytes_be(&bytes), size: 256 }),
        TokenValue::Uint(Uint{ number: BigUint::from_bytes_be(&bytes), size: 256 }),
    ];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_four_refs_and_one_int256() {
    let bytes = vec![0x55; 32];
    let bytes_builder = BuilderData::with_raw(bytes.clone(), bytes.len() * 8).unwrap();

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_reference(bytes_builder.clone());
    builder.append_reference(bytes_builder.clone());

    let mut builder_v2 = builder.clone();
    builder_v2.append_reference(bytes_builder.clone());
    builder_v2.append_builder(&bytes_builder).unwrap();

    let mut second_builder = BuilderData::new();
    second_builder.append_reference(bytes_builder.clone());
    second_builder.append_builder(&bytes_builder).unwrap();

    builder.append_reference(second_builder);

    let values = vec![
        TokenValue::Cell(bytes_builder.clone().into_cell().unwrap()),
        TokenValue::Bytes(bytes.clone()),
        TokenValue::Cell(bytes_builder.into_cell().unwrap()),
        TokenValue::Uint(Uint{ number: BigUint::from_bytes_be(&bytes), size: 256 }),
    ];

    test_parameters_set(
        &tokens_from_values(values.clone()),
        None,
        builder,
        &[ABI_VERSION_1_0],
    );

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder_v2,
        &[ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

#[test]
fn test_header_params() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    let public_key = ed25519_dalek::PublicKey::from_bytes(&[0u8; ed25519_dalek::PUBLIC_KEY_LENGTH]).unwrap();

    builder.append_bit_zero().unwrap();
    builder.append_bit_one().unwrap();
    builder.append_raw(&public_key.to_bytes(), ed25519_dalek::PUBLIC_KEY_LENGTH * 8).unwrap();
    builder.append_u64(12345).unwrap();
    builder.append_u32(67890).unwrap();

    let values = vec![
        TokenValue::PublicKey(None),
        TokenValue::PublicKey(Some(public_key)),
        TokenValue::Time(12345),
        TokenValue::Expire(67890)
    ];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
}

fn vec_to_map<K: Serializable>(vec: &[(K, BuilderData)], size: usize) -> HashmapE {
    let mut map = HashmapE::with_bit_len(size);

    for (key, value) in vec {
        let key = key.serialize().unwrap();
        map.set_builder(key.into(), &value).unwrap();
    }

    map
}

#[test]
fn test_map() {
    let bytes = vec![0x55; 32];
    let bytes_builder = BuilderData::with_raw(bytes.clone(), bytes.len() * 8).unwrap();
    let mut builder = BuilderData::new();
    builder.append_reference(bytes_builder);

    let bytes_map = vec_to_map(
        &vec![
            (1u8, builder.clone()),
            (2u8, builder.clone()),
            (3u8, builder.clone()),
        ],
        8
    );
    let bytes_value = TokenValue::Map(
        ParamType::Uint(8),
        ParamType::Bytes,
        BTreeMap::from_iter(
            vec![
                ("1".to_owned(), TokenValue::Bytes(bytes.clone())),
                ("2".to_owned(), TokenValue::Bytes(bytes.clone())),
                ("3".to_owned(), TokenValue::Bytes(bytes.clone())),
            ]
        )
    );

    let int_map = vec_to_map(
        &vec![
            (-1i16, BuilderData::with_raw((-1i128).to_be_bytes().to_vec(), 128).unwrap()),
            (0i16, BuilderData::with_raw(0i128.to_be_bytes().to_vec(), 128).unwrap()),
            (1i16, BuilderData::with_raw(1i128.to_be_bytes().to_vec(), 128).unwrap()),
        ],
        16
    );
    let int_value = TokenValue::Map(
        ParamType::Int(16),
        ParamType::Int(128),
        BTreeMap::from_iter(
            vec![
                ("-1".to_owned(), TokenValue::Int(Int::new(-1, 128))),
                ("0".to_owned(), TokenValue::Int(Int::new(0, 128))),
                ("1".to_owned(), TokenValue::Int(Int::new(1, 128))),
            ]
        )
    );

    let tuples_array: Vec<(u32, bool)> =
        vec![(1, true), (2, false), (3, true), (4, false), (5, true)];


    let bitstring_array: Vec<(u128, BuilderData)> = tuples_array
        .iter()
        .map(|a| (a.0 as u128, TupleDwordBool::from(a).write_to_new_cell().unwrap()))
        .collect();

    let tuples_map = vec_to_map(&bitstring_array, 128);

    let tuples_value = TokenValue::Map(
        ParamType::Uint(128),
        ParamType::Tuple(vec![
            Param::new("a", ParamType::Uint(32)),
            Param::new("b", ParamType::Bool),
        ]),
        BTreeMap::from_iter(
            tuples_array
                .iter()
                .map(|i| {
                    (
                        i.0.to_string(),
                        TokenValue::Tuple(tokens_from_values(vec![
                            TokenValue::Uint(Uint::new(i.0 as u128, 32)),
                            TokenValue::Bool(i.1),
                        ]))
                    )
                }),
        )
    );

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_builder(&bytes_map.write_to_new_cell().unwrap()).unwrap();
    builder.append_builder(&int_map.write_to_new_cell().unwrap()).unwrap();

    let mut builder_v2 = builder.clone();
    builder_v2.append_builder(&tuples_map.write_to_new_cell().unwrap()).unwrap();
    builder_v2.append_bit_zero().unwrap();

    let mut second_builder = BuilderData::new();
    second_builder.append_builder(&tuples_map.write_to_new_cell().unwrap()).unwrap();
    second_builder.append_bit_zero().unwrap();
    builder.append_reference(second_builder);

    let values = vec![
        bytes_value,
        int_value,
        tuples_value,
        TokenValue::Map(ParamType::Int(256), ParamType::Bool, BTreeMap::new())
    ];

    test_parameters_set(
        &tokens_from_values(values.clone()),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_2],
    );

    test_parameters_set(
        &tokens_from_values(values.clone()),
        None,
        builder_v2,
        &[ABI_VERSION_2_0],
    );
}

 #[test]
fn test_address_map_key() {
     let addr1_str = "0:1111111111111111111111111111111111111111111111111111111111111111";
     let addr2_str = "0:2222222222222222222222222222222222222222222222222222222222222222";

    let addr1 = MsgAddress::from_str(addr1_str).unwrap();
    let addr2 = MsgAddress::from_str(addr2_str).unwrap();

    let map = vec_to_map(
        &vec![
            (addr1, BuilderData::with_raw((123u32).to_be_bytes().to_vec(), 32).unwrap()),
            (addr2, BuilderData::with_raw((456u32).to_be_bytes().to_vec(), 32).unwrap()),
        ],
        crate::token::STD_ADDRESS_BIT_LENGTH);

    let value = TokenValue::Map(
        ParamType::Address,
        ParamType::Uint(32),
        BTreeMap::from_iter(
            vec![
                (addr1_str.to_owned(), TokenValue::Uint(Uint::new(123, 32))),
                (addr2_str.to_owned(), TokenValue::Uint(Uint::new(456, 32))),
            ]
        )
    );

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_builder(&map.write_to_new_cell().unwrap()).unwrap();

    test_parameters_set(
        &tokens_from_values(vec![value]),
        None,
        builder,
        &[ABI_VERSION_1_0, ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
 }

 #[test]
 fn test_big_map_value() {
    let mut map = HashmapE::with_bit_len(256);
    let mut array = HashmapE::with_bit_len(32);
    
    let mut map_value_ref = BuilderData::new();
    map_value_ref.append_u128(0).unwrap();
    map_value_ref.append_u128(4).unwrap();
    
    let mut map_value = BuilderData::new();
    map_value.append_u128(0).unwrap();
    map_value.append_u128(1).unwrap();
    map_value.append_u128(0).unwrap();
    map_value.append_u128(2).unwrap();
    map_value.append_u128(0).unwrap();
    map_value.append_u128(3).unwrap();
    map_value.append_reference(map_value_ref);

    let mut map_key = BuilderData::new();
    map_key.append_u128(0).unwrap();
    map_key.append_u128(123).unwrap();

    map.setref(map_key.into_cell().unwrap().into(), &map_value.clone().into_cell().unwrap()).unwrap();

    let mut array_key = BuilderData::new();
    array_key.append_u32(0).unwrap();

    array.setref(array_key.into_cell().unwrap().into(), &map_value.into_cell().unwrap()).unwrap();

    let tuple_tokens = tokens_from_values(vec![
        TokenValue::Uint(Uint::new(1, 256)),
        TokenValue::Uint(Uint::new(2, 256)),
        TokenValue::Uint(Uint::new(3, 256)),
        TokenValue::Uint(Uint::new(4, 256)),
    ]);
    let tuple = TokenValue::Tuple(tuple_tokens.clone());

    let value_map = TokenValue::Map(
        ParamType::Uint(256),
        ParamType::Tuple(params_from_tokens(&tuple_tokens)),
        BTreeMap::from_iter(
            vec![(
                "0x000000000000000000000000000000000000000000000000000000000000007b".to_owned(),
                tuple.clone()
            )]
        )
    );

    let value_array = TokenValue::Array(
        ParamType::Tuple(params_from_tokens(&tuple_tokens)),
        vec![tuple]
    );

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_builder(&map.write_to_new_cell().unwrap()).unwrap();
    builder.append_u32(1).unwrap();
    builder.append_builder(&array.write_to_new_cell().unwrap()).unwrap();

    test_parameters_set(
        &tokens_from_values(vec![value_map, value_array]),
        None,
        builder,
        &[ABI_VERSION_2_0, ABI_VERSION_2_2],
    );
 }

 #[test]
fn test_abi_2_1_types() {
    let string = "Some string";
    let string_builder = BuilderData::with_raw(
        string.as_bytes().to_vec(), string.as_bytes().len() * 8
    ).unwrap();
    let string_value = TokenValue::String(string.into());

    let tuple_tokens = tokens_from_values(vec![
        string_value.clone(),
        string_value.clone(),
        string_value.clone(),
        string_value.clone(),
    ]);
    let tuple = TokenValue::Tuple(tuple_tokens.clone());

    let values = vec![
        TokenValue::VarInt(16, (-123i32).into()),
        TokenValue::VarUint(32, 456u32.into()),
        TokenValue::Optional(ParamType::Bool, None),
        TokenValue::Optional(
            ParamType::Uint(1022),
            Some(Box::new(
                TokenValue::Uint(Uint::new(1, 1022))
        ))),
        TokenValue::Optional(
            ParamType::VarUint(128),
            Some(Box::new(
                TokenValue::VarUint(128, 123u32.into())
        ))),
        TokenValue::Optional(
            ParamType::Tuple(params_from_tokens(&tuple_tokens)),
            Some(Box::new(tuple))
        ),
    ];

    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    builder.append_bits(1, 4).unwrap();
    builder.append_i8(-123).unwrap();

    builder.append_bits(2, 5).unwrap();
    builder.append_u16(456).unwrap();

    builder.append_bit_zero().unwrap();

    let mut uint_builder = BuilderData::new();
    uint_builder.append_bit_one().unwrap();
    uint_builder.append_raw(&[0u8; 127], 127 * 8).unwrap();
    uint_builder.append_raw(&[0x4], 6).unwrap();

    let mut varuint_builder = BuilderData::new();
    varuint_builder.append_raw(&[0x2], 7).unwrap();
    varuint_builder.append_u8(123).unwrap();
    let mut varuint_builder = BuilderData::with_raw_and_refs(
        vec![0x80],
        1,
        vec![varuint_builder.into_cell().unwrap()]
    ).unwrap();

    let tuple_builder = BuilderData::with_raw_and_refs(
        vec![],
        0,
        vec![
            string_builder.clone().into_cell().unwrap(),
            string_builder.clone().into_cell().unwrap(),
            string_builder.clone().into_cell().unwrap(),
            string_builder.clone().into_cell().unwrap(),
        ]
    ).unwrap();
    let tuple_builder = BuilderData::with_raw_and_refs(
        vec![0x80],
        1,
        vec![tuple_builder.into_cell().unwrap()]
    ).unwrap();

    varuint_builder.append_builder(&tuple_builder).unwrap();
    uint_builder.checked_append_reference(varuint_builder.into_cell().unwrap()).unwrap();
    builder.checked_append_reference(uint_builder.into_cell().unwrap()).unwrap();

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[ABI_VERSION_2_1, ABI_VERSION_2_2],
    );
 }

 #[test]
fn test_ref_type() {
    // test prefix with one ref and u32
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(BuilderData::new());

    let mut ref_builder = BuilderData::new();
    ref_builder.append_bit_one().unwrap();
    ref_builder.append_reference(BuilderData::new());

    builder.append_reference(123u64.write_to_new_cell().unwrap());
    builder.append_reference(ref_builder.clone());

    let values = vec![
        TokenValue::Ref(Box::new(TokenValue::Int(Int::new(123, 64)))),
        TokenValue::Ref(Box::new(TokenValue::Tuple(tokens_from_values(vec![
            TokenValue::Bool(true),
            TokenValue::Cell(Cell::default()),
        ])))),
    ];

    test_parameters_set(
        &tokens_from_values(values),
        None,
        builder,
        &[MAX_SUPPORTED_VERSION],
    );
}

#[test]
fn test_partial_decoding() {
    let mut builder = BuilderData::new();
    builder.append_u32(0).unwrap();
    builder.append_reference(123u64.write_to_new_cell().unwrap());
    builder.append_bit_one().unwrap();

    let params = vec![
        Param::new("a", ParamType::Uint(32)),
        Param::new("b", ParamType::Ref(Box::new(ParamType::Int(32)))),
        Param::new("c", ParamType::Bool),
    ];

    assert!(TokenValue::decode_params(&params, builder.clone().into_cell().unwrap().into(), &MAX_SUPPORTED_VERSION, false).is_err());

    let params = vec![
        Param::new("a", ParamType::Uint(32)),
        Param::new("b", ParamType::Ref(Box::new(ParamType::Int(64)))),
    ];

    assert!(TokenValue::decode_params(&params, builder.clone().into_cell().unwrap().into(), &MAX_SUPPORTED_VERSION, false).is_err());

    assert_eq!(
        TokenValue::decode_params(&params, builder.into_cell().unwrap().into(), &MAX_SUPPORTED_VERSION, true).unwrap(),
        tokens_from_values(vec![
            TokenValue::Uint(Uint::new(0, 32)),
            TokenValue::Ref(Box::new(TokenValue::Int(Int::new(123, 64)))),
        ])
    );
}
