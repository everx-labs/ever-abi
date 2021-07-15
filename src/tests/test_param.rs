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

use token::Detokenizer;
use Int;

use Token;
use {Param, ParamType};
use {TokenValue, Uint};
use Function;
use ton_types::BuilderData;
use ton_types::IBitstring;

use crate::contract::ABI_VBERSION_2_0;

#[test]
fn int_json_representation() {
    let value = Detokenizer::detokenize_to_json_value(
        &[
            Token::new("u8", TokenValue::Uint(Uint::new(1, 8))),
            Token::new("i32", TokenValue::Int(Int::new(-1, 32))),
            Token::new("u256", TokenValue::Uint(Uint::new(1, 256))),
            Token::new("u128", TokenValue::Uint(Uint::new(1, 128))),
            Token::new("i256", TokenValue::Int(Int::new(-1, 256))),
            Token::new("vi16", TokenValue::VarInt(16, (-1i32).into())),
            Token::new("vu32", TokenValue::VarUint(32, 1u32.into())),
        ],
    )
    .unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "u8": "1",
            "i32": "-1",
            "u256": "0x0000000000000000000000000000000000000000000000000000000000000001",
            "u128": "1",
            "i256": "-1",
            "vi16": "-1",
            "vu32": "1",
        })
    );
}

#[test]
fn test_encode_internal_output() {
    let func: Function = Function {
        abi_version: ABI_VBERSION_2_0,
        name: "func".to_string(),
        header: vec![],
        inputs: vec![],
        outputs: vec![],
        input_id: 0,
        output_id: 0,
    };

    let tokens =
        [
            Token::new("u8", TokenValue::Uint(Uint::new(1, 8))),
            Token::new("i32", TokenValue::Int(Int::new(-1, 32))),
            Token::new("u256", TokenValue::Uint(Uint::new(1, 256))),
            Token::new("u128", TokenValue::Uint(Uint::new(1, 128))),
            Token::new("i256", TokenValue::Int(Int::new(-1, 256))),
        ];
    let test_tree = func.encode_internal_output(1u32 << 31, &tokens).unwrap();

    let mut expected_tree = BuilderData::new();
    expected_tree.append_u32(1u32 << 31).unwrap();        // answer_id
    expected_tree.append_u8(1).unwrap();
    expected_tree.append_i32(-1).unwrap();
    expected_tree.append_raw(
        &hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap(),
        32 * 8).unwrap();
    expected_tree.append_raw(
        &hex::decode("00000000000000000000000000000001").unwrap(),
        16 * 8).unwrap();
    expected_tree.append_raw(
        &hex::decode("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap(),
        32 * 8).unwrap();
    assert_eq!(test_tree, expected_tree);
}

#[test]
fn test_simple_param_deserialization() {
    let s = r#"{
        "name": "a",
        "type": "int9"
    }"#;

    let deserialized: Param = serde_json::from_str(s).unwrap();

    assert_eq!(deserialized, Param {
        name: "a".to_owned(),
        kind: ParamType::Int(9),
    });
}

#[test]
fn test_tuple_param_deserialization() {
    let s = r#"{
        "name": "a",
        "type": "tuple",
        "components" : [
            {
                "name" : "a",
                "type" : "int8"
            },
            {
                "name" : "b",
                "type" : "int8"
            }
        ]
    }"#;

    let deserialized: Param = serde_json::from_str(s).unwrap();

    assert_eq!(deserialized, Param {
        name: "a".to_owned(),
        kind: ParamType::Tuple(vec![
            Param { name: "a".to_owned(), kind: ParamType::Int(8) },
            Param { name: "b".to_owned(), kind: ParamType::Int(8) },
        ]),
    });
}

#[test]
fn test_tuples_array_deserialization() {
    let s = r#"{
        "name": "a",
        "type": "tuple[]",
        "components" : [
            {
                "name" : "a",
                "type" : "bool"
            },
            {
                "name" : "b",
                "type" : "tuple[5]",
                "components" : [
                    {
                        "name" : "a",
                        "type" : "uint8"
                    },
                    {
                        "name" : "b",
                        "type" : "int15"
                    }
                ]
            }
        ]
    }"#;

    let deserialized: Param = serde_json::from_str(s).unwrap();

    assert_eq!(deserialized, Param {
        name: "a".to_owned(),
        kind: ParamType::Array(Box::new(ParamType::Tuple(vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Bool
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::FixedArray(
                    Box::new(ParamType::Tuple(vec![
                        Param { name: "a".to_owned(), kind: ParamType::Uint(8) },
                        Param { name: "b".to_owned(), kind: ParamType::Int(15) },
                    ])),
                    5
                )
            },
        ]))),
    });
}

#[test]
fn test_tuples_array_map_map() {
    let s = r#"{
        "components":[
            {
                "name": "a",
                "type": "uint256"
            },
            {
                "name": "b",
                "type": "uint256"
            }
        ],
        "name": "d",
        "type": "map(uint32,map(uint32,tuple[][5]))"
    }"#;

    let deserialized: Param = serde_json::from_str(s).unwrap();

    assert_eq!(deserialized, Param {
        name: "d".to_owned(),
        kind: ParamType::Map(
                Box::new(ParamType::Uint(32)),
                Box::new(ParamType::Map(
                    Box::new(ParamType::Uint(32)),
                    Box::new(ParamType::FixedArray(
                        Box::new(ParamType::Array(
                            Box::new(ParamType::Tuple(vec![
                                Param { name: "a".to_owned(), kind: ParamType::Uint(256) },
                                Param { name: "b".to_owned(), kind: ParamType::Uint(256) },
                            ]))
                        )),
                        5
                    )),
                ))
            ),
    });
}

#[test]
fn test_empty_tuple_error() {
    let s = r#"{
        "name": "a",
        "type": "map(uint256,tuple)"
    }"#;

    let result = serde_json::from_str::<Param>(s).unwrap_err();

    assert_eq!(
        "Tuple description should contain non empty `components` field",
        format!("{}", result)
    )
}

#[test]
fn test_optional_tuple_param_deserialization() {
    let s = r#"{
        "name": "a",
        "type": "optional(tuple)",
        "components" : [
            { "name" : "a", "type" : "int8" },
            { "name" : "b", "type" : "int8" }
        ]
    }"#;

    let deserialized: Param = serde_json::from_str(s).unwrap();

    assert_eq!(deserialized, Param {
        name: "a".to_owned(),
        kind: ParamType::Optional(Box::new(ParamType::Tuple(vec![
            Param { name: "a".to_owned(), kind: ParamType::Int(8) },
            Param { name: "b".to_owned(), kind: ParamType::Int(8) },
        ]))),
    });
}
