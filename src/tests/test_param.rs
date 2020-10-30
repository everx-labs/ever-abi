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

#[test]
fn int_json_representation() {
    let value = Detokenizer::detokenize_to_json_value(
        &[
            Param::new("u8", ParamType::Uint(8)),
            Param::new("i32", ParamType::Int(32)),
            Param::new("u256", ParamType::Uint(256)),
            Param::new("u128", ParamType::Uint(128)),
            Param::new("i256", ParamType::Int(256)),
        ],
        &[
            Token::new("u8", TokenValue::Uint(Uint::new(1, 8))),
            Token::new("i32", TokenValue::Int(Int::new(-1, 32))),
            Token::new("u256", TokenValue::Uint(Uint::new(1, 256))),
            Token::new("u128", TokenValue::Uint(Uint::new(1, 128))),
            Token::new("i256", TokenValue::Int(Int::new(-1, 256))),
        ],
    )
    .unwrap();
    json!({});
    assert_eq!(
        value,
        json!({
            "u8": "1",
            "i32": "-1",
            "u256": "0x0000000000000000000000000000000000000000000000000000000000000001",
            "u128": "1",
            "i256": "-1",
        })
    );
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
