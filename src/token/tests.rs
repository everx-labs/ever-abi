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

mod tokenize_tests {
    use crate::token::{Detokenizer, Tokenizer};
    use crate::{Int, Param, ParamType, Token, TokenValue, Uint};
    use std::collections::BTreeMap;
    use ever_block::{Grams, MsgAddress};
    use ever_block::{AccountId, BuilderData, Cell, SliceData, ED25519_PUBLIC_KEY_LENGTH};

    #[test]
    fn test_tokenize_ints() {
        let max_gram = 0x007F_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFFu128; // 2^120 - 1
        let input = r#"{
            "a" : 123,
            "b" : -456,
            "c" : "-0xabcdef",
            "e" : "789",
            "f" : "-12345678900987654321",
            "g" : "0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
            "h" : "-1000",
            "i" : "1000"
        }"#;

        let params = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Uint(8),
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Int(16),
            },
            Param {
                name: "c".to_owned(),
                kind: ParamType::Int(32),
            },
            Param {
                name: "e".to_owned(),
                kind: ParamType::Uint(13),
            },
            Param {
                name: "f".to_owned(),
                kind: ParamType::Int(128),
            },
            Param {
                name: "g".to_owned(),
                kind: ParamType::Token,
            },
            Param {
                name: "h".to_owned(),
                kind: ParamType::VarInt(16),
            },
            Param {
                name: "i".to_owned(),
                kind: ParamType::VarUint(32),
            },
        ];

        let expected_tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Uint(Uint::new(123, 8)),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::Int(Int::new(-456, 16)),
            },
            Token {
                name: "c".to_owned(),
                value: TokenValue::Int(Int::new(-0xabcdef, 32)),
            },
            Token {
                name: "e".to_owned(),
                value: TokenValue::Uint(Uint::new(789, 13)),
            },
            Token {
                name: "f".to_owned(),
                value: TokenValue::Int(Int::new(-12345678900987654321i128, 128)),
            },
            Token::new("g", TokenValue::Token(Grams::new(max_gram).unwrap())),
            Token {
                name: "h".to_owned(),
                value: TokenValue::VarInt(16, (-1000i32).into()),
            },
            Token {
                name: "i".to_owned(),
                value: TokenValue::VarUint(32, 1000u32.into()),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_int_checks() {
        // number doesn't fit into parameter size
        let input = r#"{ "a" : 128 }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Uint(7),
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).is_err()
        );

        // number doesn't fit into i64 range used in serde_json
        let input = r#"{ "a" : 12345678900987654321 }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Int(64),
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).is_err()
        );

        // test BigInt::bits() case for -2^n values

        let input_fit = r#"{ "a" : -128 }"#;
        let input_not_fit = r#"{ "a" : -129 }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Int(8),
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_fit).unwrap())
                .is_ok()
        );
        assert!(Tokenizer::tokenize_all_params(
            &params,
            &serde_json::from_str(input_not_fit).unwrap()
        )
        .is_err());

        // negative values for uint
        let input_num = r#"{ "a" : -1 }"#;
        let input_str = r#"{ "a" : "-5" }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Uint(8),
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_num).unwrap())
                .is_err()
        );
        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_str).unwrap())
                .is_err()
        );

        // varint max check
        let input = r#"{ "a" : "0xffffffffffffffffffffffffffffffff" }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::VarInt(16),
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).is_err()
        );

        // negative values for varuint
        let input_num = r#"{ "a" : -1 }"#;
        let input_str = r#"{ "a" : "-5" }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::VarUint(8),
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_num).unwrap())
                .is_err()
        );
        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_str).unwrap())
                .is_err()
        );
    }

    #[test]
    fn test_tokenize_bool() {
        let input = r#"{
            "a" : true,
            "b" : "false"
        }"#;

        let params = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Bool,
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Bool,
            },
        ];

        let expected_tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Bool(true),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::Bool(false),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_empty() {
        let input = r#"{}"#;

        let params = vec![];

        let expected_tokens = vec![];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_arrays() {
        let input = r#"{
            "a" : [123, -456, "789", "-0x0ABc"],
            "b" : [
                [false, "true"],
                [true, true, false]
            ]
        }"#;

        let params = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Array(Box::new(ParamType::Int(16))),
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::FixedArray(
                    Box::new(ParamType::Array(Box::new(ParamType::Bool))),
                    2,
                ),
            },
        ];

        let dint_array = vec![
            TokenValue::Int(Int::new(123, 16)),
            TokenValue::Int(Int::new(-456, 16)),
            TokenValue::Int(Int::new(789, 16)),
            TokenValue::Int(Int::new(-0x0abc, 16)),
        ];

        let bool_array1 = vec![TokenValue::Bool(false), TokenValue::Bool(true)];

        let bool_array2 = vec![
            TokenValue::Bool(true),
            TokenValue::Bool(true),
            TokenValue::Bool(false),
        ];

        let expected_tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Array(ParamType::Int(16), dint_array),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::FixedArray(
                    ParamType::Array(Box::new(ParamType::Bool)),
                    vec![
                        TokenValue::Array(ParamType::Bool, bool_array1),
                        TokenValue::Array(ParamType::Bool, bool_array2),
                    ],
                ),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_tuple() {
        let input = r#"{
            "t1" : {
                "a" : [-123, "456", "0x789"],
                "b" : "false",
                "c" : "0x1234"
            },
            "t2" : [
                {
                    "a" : true,
                    "b" : "0x12"
                },
                {
                    "a" : false,
                    "b" : "0x34"
                },
                {
                    "a" : true,
                    "b" : "0x56"
                }
            ]
        }"#;

        let tuple_params1 = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Array(Box::new(ParamType::Int(16))),
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Bool,
            },
            Param {
                name: "c".to_owned(),
                kind: ParamType::Int(16),
            },
        ];

        let tuple_params2 = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Bool,
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Int(8),
            },
        ];

        let params = vec![
            Param {
                name: "t1".to_owned(),
                kind: ParamType::Tuple(tuple_params1),
            },
            Param {
                name: "t2".to_owned(),
                kind: ParamType::Array(Box::new(ParamType::Tuple(tuple_params2))),
            },
        ];

        let expected_tokens = vec![
            Token {
                name: "t1".to_owned(),
                value: TokenValue::Tuple(vec![
                    Token {
                        name: "a".to_owned(),
                        value: TokenValue::Array(
                            ParamType::Int(16),
                            vec![
                                TokenValue::Int(Int::new(-123, 16)),
                                TokenValue::Int(Int::new(456, 16)),
                                TokenValue::Int(Int::new(0x789, 16)),
                            ],
                        ),
                    },
                    Token {
                        name: "b".to_owned(),
                        value: TokenValue::Bool(false),
                    },
                    Token {
                        name: "c".to_owned(),
                        value: TokenValue::Int(Int::new(0x1234, 16)),
                    },
                ]),
            },
            Token {
                name: "t2".to_owned(),
                value: TokenValue::Array(
                    ParamType::Tuple(vec![
                        Param {
                            name: "a".to_owned(),
                            kind: ParamType::Bool,
                        },
                        Param {
                            name: "b".to_owned(),
                            kind: ParamType::Int(8),
                        },
                    ]),
                    vec![
                        TokenValue::Tuple(vec![
                            Token {
                                name: "a".to_owned(),
                                value: TokenValue::Bool(true),
                            },
                            Token {
                                name: "b".to_owned(),
                                value: TokenValue::Int(Int::new(0x12, 8)),
                            },
                        ]),
                        TokenValue::Tuple(vec![
                            Token {
                                name: "a".to_owned(),
                                value: TokenValue::Bool(false),
                            },
                            Token {
                                name: "b".to_owned(),
                                value: TokenValue::Int(Int::new(0x34, 8)),
                            },
                        ]),
                        TokenValue::Tuple(vec![
                            Token {
                                name: "a".to_owned(),
                                value: TokenValue::Bool(true),
                            },
                            Token {
                                name: "b".to_owned(),
                                value: TokenValue::Int(Int::new(0x56, 8)),
                            },
                        ]),
                    ],
                ),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_cell() {
        let input = r#"{
            "a": "te6ccgEBAwEAIAACEAECAwQFBgcIAgEAEBUWFxgZGhscABALDA0ODxAREg==",
            "b": ""
        }"#;

        let params = vec![
            Param::new("a", ParamType::Cell),
            Param::new("b", ParamType::Cell),
        ];

        let mut expected_tokens = vec![];
        let mut builder = BuilderData::with_bitstring(vec![1, 2, 3, 4, 5, 6, 7, 8, 0x80]).unwrap();
        builder
            .checked_append_reference(
                BuilderData::with_bitstring(vec![11, 12, 13, 14, 15, 16, 17, 18, 0x80])
                    .unwrap()
                    .into_cell()
                    .unwrap(),
            )
            .unwrap();
        builder
            .checked_append_reference(
                BuilderData::with_bitstring(vec![21, 22, 23, 24, 25, 26, 27, 28, 0x80])
                    .unwrap()
                    .into_cell()
                    .unwrap(),
            )
            .unwrap();
        expected_tokens.push(Token::new(
            "a",
            TokenValue::Cell(builder.into_cell().unwrap()),
        ));
        expected_tokens.push(Token::new("b", TokenValue::Cell(Cell::default())));

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_hashmap() {
        let input = r#"{
            "a": {
                "-12": 42,
                "127": 37,
                "-128": 56
            },
            "b": {
                "4294967295": 777,
                "65535": 0
            },
            "c": {
                "1": {
                    "q1" : 314,
                    "q2" : 15
                },
                "2": {
                    "q1" : 92,
                    "q2" : 6
                }
            },
            "d": {
                "0:1111111111111111111111111111111111111111111111111111111111111111": 123
            }
        }"#;

        let params = vec![
            Param::new(
                "a",
                ParamType::Map(Box::new(ParamType::Int(8)), Box::new(ParamType::Uint(32))),
            ),
            Param::new(
                "b",
                ParamType::Map(Box::new(ParamType::Uint(32)), Box::new(ParamType::Uint(32))),
            ),
            Param::new(
                "c",
                ParamType::Map(
                    Box::new(ParamType::Int(8)),
                    Box::new(ParamType::Tuple(vec![
                        Param::new("q1", ParamType::Uint(32)),
                        Param::new("q2", ParamType::Int(8)),
                    ])),
                ),
            ),
            Param::new(
                "d",
                ParamType::Map(Box::new(ParamType::Address), Box::new(ParamType::Uint(32))),
            ),
        ];

        let mut expected_tokens = vec![];
        let mut map = BTreeMap::<String, TokenValue>::new();
        map.insert(format!("{}", -12i8), TokenValue::Uint(Uint::new(42, 32)));
        map.insert(format!("{}", 127i8), TokenValue::Uint(Uint::new(37, 32)));
        map.insert(format!("{}", -128i8), TokenValue::Uint(Uint::new(56, 32)));
        expected_tokens.push(Token::new(
            "a",
            TokenValue::Map(ParamType::Int(8), ParamType::Uint(32), map),
        ));

        let mut map = BTreeMap::<String, TokenValue>::new();
        map.insert(
            format!("{}", 0xFFFFFFFFu32),
            TokenValue::Uint(Uint::new(777, 32)),
        );
        map.insert(
            format!("{}", 0x0000FFFFu32),
            TokenValue::Uint(Uint::new(0, 32)),
        );
        expected_tokens.push(Token::new(
            "b",
            TokenValue::Map(ParamType::Uint(32), ParamType::Uint(32), map),
        ));

        let mut map = BTreeMap::<String, TokenValue>::new();
        map.insert(
            format!("{}", 1i8),
            TokenValue::Tuple(vec![
                Token::new("q1", TokenValue::Uint(Uint::new(314, 32))),
                Token::new("q2", TokenValue::Int(Int::new(15, 8))),
            ]),
        );
        map.insert(
            format!("{}", 2i8),
            TokenValue::Tuple(vec![
                Token::new("q1", TokenValue::Uint(Uint::new(92, 32))),
                Token::new("q2", TokenValue::Int(Int::new(6, 8))),
            ]),
        );
        expected_tokens.push(Token::new(
            "c",
            TokenValue::Map(
                ParamType::Int(8),
                ParamType::Tuple(vec![
                    Param {
                        name: "q1".to_owned(),
                        kind: ParamType::Uint(32),
                    },
                    Param {
                        name: "q2".to_owned(),
                        kind: ParamType::Int(8),
                    },
                ]),
                map,
            ),
        ));

        let mut map = BTreeMap::<String, TokenValue>::new();
        map.insert(
            format!(
                "{}",
                MsgAddress::with_standart(None, 0, AccountId::from([0x11; 32])).unwrap()
            ),
            TokenValue::Uint(Uint::new(123, 32)),
        );
        expected_tokens.push(Token::new(
            "d",
            TokenValue::Map(ParamType::Address, ParamType::Uint(32), map),
        ));

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_address() {
        let input = r#"{
            "std": "-17:5555555555555555555555555555555555555555555555555555555555555555",
            "var": "-177:555_"
        }"#;

        let params = vec![
            Param::new("std", ParamType::Address),
            Param::new("var", ParamType::Address),
        ];

        let expected_tokens = vec![
            Token {
                name: "std".to_owned(),
                value: TokenValue::Address(
                    MsgAddress::with_standart(None, -17, AccountId::from([0x55; 32])).unwrap(),
                ),
            },
            Token {
                name: "var".to_owned(),
                value: TokenValue::Address(
                    MsgAddress::with_variant(None, -177, SliceData::new(vec![0x55, 0x50])).unwrap(),
                ),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_bytes() {
        let input = r#"{
            "a": "ABCDEF",
            "b": "ABCDEF0102",
            "c": "55555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555"
        }"#;

        let params = vec![
            Param::new("a", ParamType::Bytes),
            Param::new("b", ParamType::FixedBytes(3)),
            Param::new("c", ParamType::Bytes),
        ];

        let expected_tokens = vec![
            Token::new("a", TokenValue::Bytes(vec![0xAB, 0xCD, 0xEF])),
            Token::new("b", TokenValue::FixedBytes(vec![0xAB, 0xCD, 0xEF])),
            Token::new("c", TokenValue::Bytes(vec![0x55; 160])),
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_time() {
        let input = r#"{
            "a" : 123,
            "b" : "456",
            "c" : "0x789",
            "d": "0xffffffffffffffff"
        }"#;

        let params = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Time,
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Time,
            },
            Param {
                name: "c".to_owned(),
                kind: ParamType::Time,
            },
            Param {
                name: "d".to_owned(),
                kind: ParamType::Time,
            },
        ];

        let expected_tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Time(123),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::Time(456),
            },
            Token {
                name: "c".to_owned(),
                value: TokenValue::Time(0x789),
            },
            Token {
                name: "d".to_owned(),
                value: TokenValue::Time(0xffffffffffffffff),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_time_checks() {
        // number doesn't fit into parameter size
        let input = r#"{ "a" : "0x10000000000000000" }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Time,
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).is_err()
        );

        // negative values for time
        let input_num = r#"{ "a" : -1 }"#;
        let input_str = r#"{ "a" : "-5" }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Time,
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_num).unwrap())
                .is_err()
        );
        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_str).unwrap())
                .is_err()
        );
    }

    #[test]
    fn test_tokenize_expire() {
        let input = r#"{
            "a" : 123,
            "b" : "456",
            "c" : "0x789",
            "d": "0xffffffff"
        }"#;

        let params = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Expire,
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Expire,
            },
            Param {
                name: "c".to_owned(),
                kind: ParamType::Expire,
            },
            Param {
                name: "d".to_owned(),
                kind: ParamType::Expire,
            },
        ];

        let expected_tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Expire(123),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::Expire(456),
            },
            Token {
                name: "c".to_owned(),
                value: TokenValue::Expire(0x789),
            },
            Token {
                name: "d".to_owned(),
                value: TokenValue::Expire(0xffffffff),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_expire_checks() {
        // number doesn't fit into parameter size
        let input = r#"{ "a" : "0x100000000" }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Expire,
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).is_err()
        );

        // negative values for expire
        let input_num = r#"{ "a" : -1 }"#;
        let input_str = r#"{ "a" : "-5" }"#;
        let params = vec![Param {
            name: "a".to_owned(),
            kind: ParamType::Expire,
        }];

        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_num).unwrap())
                .is_err()
        );
        assert!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input_str).unwrap())
                .is_err()
        );
    }

    #[test]
    fn test_tokenize_pubkey() {
        let input = r#"{
            "a": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "b": ""
        }"#;

        let params = vec![
            Param::new("a", ParamType::PublicKey),
            Param::new("b", ParamType::PublicKey),
        ];

        let expected_tokens = vec![
            Token::new(
                "a",
                TokenValue::PublicKey(Some([0xcc; ED25519_PUBLIC_KEY_LENGTH])),
            ),
            Token::new("b", TokenValue::PublicKey(None)),
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );

        // check that detokenizer gives the same result
        let input = Detokenizer::detokenize(&expected_tokens).unwrap();
        println!("{}", input);
        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(&input).unwrap())
                .unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_optional() {
        let input = r#"{
            "a": 123,
            "b": null
        }"#;

        let params = vec![
            Param::new("a", ParamType::Optional(Box::new(ParamType::VarUint(32)))),
            Param::new("b", ParamType::Optional(Box::new(ParamType::VarUint(32)))),
            Param::new("c", ParamType::Optional(Box::new(ParamType::VarUint(32)))),
        ];

        let expected_tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Optional(
                    ParamType::VarUint(32),
                    Some(Box::new(TokenValue::VarUint(32, 123u32.into()))),
                ),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::Optional(ParamType::VarUint(32), None),
            },
            Token {
                name: "c".to_owned(),
                value: TokenValue::Optional(ParamType::VarUint(32), None),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_tokenize_ref() {
        let input = r#"{
            "a": 123,
            "b": {
                "c": true,
                "d": "some string"
            }
        }"#;

        let params = vec![
            Param::new("a", ParamType::Ref(Box::new(ParamType::VarUint(32)))),
            Param::new(
                "b",
                ParamType::Ref(Box::new(ParamType::Tuple(vec![
                    Param::new("c", ParamType::Bool),
                    Param::new("d", ParamType::String),
                ]))),
            ),
        ];

        let expected_tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Ref(Box::new(TokenValue::VarUint(32, 123u32.into()))),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::Ref(Box::new(TokenValue::Tuple(vec![
                    Token {
                        name: "c".to_owned(),
                        value: TokenValue::Bool(true),
                    },
                    Token {
                        name: "d".to_owned(),
                        value: TokenValue::String("some string".to_owned()),
                    },
                ]))),
            },
        ];

        assert_eq!(
            Tokenizer::tokenize_all_params(&params, &serde_json::from_str(input).unwrap()).unwrap(),
            expected_tokens
        );
    }

    #[test]
    fn test_unknown_param() {
        let input = r#"{
            "a": 123,
            "b": 456
        }"#;

        let params = vec![Param::new("a", ParamType::Time)];

        assert!(Tokenizer::tokenize_optional_params(
            &params,
            &serde_json::from_str(input).unwrap(),
        )
        .is_err(),);
    }
}

mod types_check_tests {
    use crate::{Int, Param, ParamType, Token, TokenValue, Uint};
    use std::collections::BTreeMap;
    use ever_block::MsgAddress;
    use ever_block::Cell;

    #[test]
    fn test_type_check() {
        fn assert_type_check(tokens: &[Token], params: &[Param]) {
            assert!(Token::types_check(&tokens, params))
        }

        fn assert_not_type_check(tokens: &[Token], params: &[Param]) {
            assert!(!Token::types_check(&tokens, params))
        }

        let big_int = Int::new(123, 64);
        let big_uint = Uint::new(456, 32);
        let mut map = BTreeMap::<String, TokenValue>::new();
        map.insert("1".to_string(), TokenValue::Uint(Uint::new(17, 32)));

        let tokens = vec![
            Token {
                name: "a".to_owned(),
                value: TokenValue::Uint(big_uint.clone()),
            },
            Token {
                name: "b".to_owned(),
                value: TokenValue::Int(big_int.clone()),
            },
            Token {
                name: "c".to_owned(),
                value: TokenValue::VarUint(32, 789u32.into()),
            },
            Token {
                name: "d".to_owned(),
                value: TokenValue::VarInt(16, 1000u32.into()),
            },
            Token {
                name: "e".to_owned(),
                value: TokenValue::Bool(false),
            },
            Token {
                name: "f".to_owned(),
                value: TokenValue::Array(
                    ParamType::Bool,
                    vec![TokenValue::Bool(false), TokenValue::Bool(true)],
                ),
            },
            Token {
                name: "g".to_owned(),
                value: TokenValue::FixedArray(
                    ParamType::Int(64),
                    vec![
                        TokenValue::Int(big_int.clone()),
                        TokenValue::Int(big_int.clone()),
                    ],
                ),
            },
            Token {
                name: "j".to_owned(),
                value: TokenValue::Tuple(vec![
                    Token {
                        name: "a".to_owned(),
                        value: TokenValue::Bool(true),
                    },
                    Token {
                        name: "b".to_owned(),
                        value: TokenValue::Uint(big_uint.clone()),
                    },
                ]),
            },
            Token {
                name: "k".to_owned(),
                value: TokenValue::Cell(Cell::default()),
            },
            Token {
                name: "l".to_owned(),
                value: TokenValue::Address(MsgAddress::AddrNone),
            },
            Token {
                name: "m1".to_owned(),
                value: TokenValue::Map(
                    ParamType::Int(8),
                    ParamType::Bool,
                    BTreeMap::<String, TokenValue>::new(),
                ),
            },
            Token {
                name: "m2".to_owned(),
                value: TokenValue::Map(ParamType::Int(8), ParamType::Uint(32), map),
            },
            Token {
                name: "n".to_owned(),
                value: TokenValue::Bytes(vec![1]),
            },
            Token {
                name: "o".to_owned(),
                value: TokenValue::FixedBytes(vec![1, 2, 3]),
            },
            Token {
                name: "p".to_owned(),
                value: TokenValue::Token(17u64.into()),
            },
            Token {
                name: "q".to_owned(),
                value: TokenValue::Time(123),
            },
            Token {
                name: "r".to_owned(),
                value: TokenValue::Expire(456),
            },
            Token {
                name: "s".to_owned(),
                value: TokenValue::PublicKey(None),
            },
            Token {
                name: "t".to_owned(),
                value: TokenValue::String("123".to_owned()),
            },
            Token {
                name: "u".to_owned(),
                value: TokenValue::Optional(ParamType::Int(256), None),
            },
            Token {
                name: "v".to_owned(),
                value: TokenValue::Optional(
                    ParamType::Bool,
                    Some(Box::new(TokenValue::Bool(true))),
                ),
            },
            Token {
                name: "w".to_owned(),
                value: TokenValue::Ref(Box::new(TokenValue::String("123".to_owned()))),
            },
        ];

        let tuple_params = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Bool,
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Uint(32),
            },
        ];

        let params = vec![
            Param {
                name: "a".to_owned(),
                kind: ParamType::Uint(32),
            },
            Param {
                name: "b".to_owned(),
                kind: ParamType::Int(64),
            },
            Param {
                name: "c".to_owned(),
                kind: ParamType::VarUint(32),
            },
            Param {
                name: "d".to_owned(),
                kind: ParamType::VarInt(16),
            },
            Param {
                name: "e".to_owned(),
                kind: ParamType::Bool,
            },
            Param {
                name: "f".to_owned(),
                kind: ParamType::Array(Box::new(ParamType::Bool)),
            },
            Param {
                name: "g".to_owned(),
                kind: ParamType::FixedArray(Box::new(ParamType::Int(64)), 2),
            },
            Param {
                name: "j".to_owned(),
                kind: ParamType::Tuple(tuple_params),
            },
            Param {
                name: "k".to_owned(),
                kind: ParamType::Cell,
            },
            Param {
                name: "l".to_owned(),
                kind: ParamType::Address,
            },
            Param {
                name: "m1".to_owned(),
                kind: ParamType::Map(Box::new(ParamType::Int(8)), Box::new(ParamType::Bool)),
            },
            Param {
                name: "m2".to_owned(),
                kind: ParamType::Map(Box::new(ParamType::Int(8)), Box::new(ParamType::Uint(32))),
            },
            Param {
                name: "n".to_owned(),
                kind: ParamType::Bytes,
            },
            Param {
                name: "o".to_owned(),
                kind: ParamType::FixedBytes(3),
            },
            Param {
                name: "p".to_owned(),
                kind: ParamType::Token,
            },
            Param {
                name: "q".to_owned(),
                kind: ParamType::Time,
            },
            Param {
                name: "r".to_owned(),
                kind: ParamType::Expire,
            },
            Param {
                name: "s".to_owned(),
                kind: ParamType::PublicKey,
            },
            Param {
                name: "t".to_owned(),
                kind: ParamType::String,
            },
            Param {
                name: "u".to_owned(),
                kind: ParamType::Optional(Box::new(ParamType::Int(256))),
            },
            Param {
                name: "v".to_owned(),
                kind: ParamType::Optional(Box::new(ParamType::Bool)),
            },
            Param {
                name: "w".to_owned(),
                kind: ParamType::Ref(Box::new(ParamType::String)),
            },
        ];

        assert_type_check(&tokens, &params);

        let mut tokens_wrong_type = tokens.clone();
        tokens_wrong_type[0] = Token {
            name: "a".to_owned(),
            value: TokenValue::Bool(false),
        };
        assert_not_type_check(&tokens_wrong_type, &params);

        let mut tokens_wrong_int_size = tokens.clone();
        tokens_wrong_int_size[0] = Token {
            name: "a".to_owned(),
            value: TokenValue::Uint(Uint::new(456, 30)),
        };
        assert_not_type_check(&tokens_wrong_int_size, &params);

        let mut tokens_wrong_parameters_count = tokens.clone();
        tokens_wrong_parameters_count.pop();
        assert_not_type_check(&tokens_wrong_parameters_count, &params);

        let mut tokens_wrong_fixed_array_size = tokens.clone();
        tokens_wrong_fixed_array_size[6] = Token {
            name: "g".to_owned(),
            value: TokenValue::FixedArray(
                ParamType::Int(64),
                vec![TokenValue::Int(big_int.clone())],
            ),
        };
        assert_not_type_check(&tokens_wrong_fixed_array_size, &params);

        let mut tokens_wrong_array_type = tokens.clone();
        tokens_wrong_array_type[5] = Token {
            name: "f".to_owned(),
            value: TokenValue::Array(
                ParamType::Bool,
                vec![TokenValue::Bool(false), TokenValue::Int(big_int.clone())],
            ),
        };
        assert_not_type_check(&tokens_wrong_array_type, &params);

        let mut tokens_wrong_tuple_type = tokens.clone();
        tokens_wrong_tuple_type[9] = Token {
            name: "f".to_owned(),
            value: TokenValue::Tuple(vec![
                Token {
                    name: "a".to_owned(),
                    value: TokenValue::Int(big_int.clone()),
                },
                Token {
                    name: "b".to_owned(),
                    value: TokenValue::Uint(big_uint.clone()),
                },
            ]),
        };
        assert_not_type_check(&tokens_wrong_tuple_type, &params);
    }
}

mod default_values_tests {
    use crate::{ParamType, TokenValue};
    use chrono::prelude::Utc;

    #[test]
    fn test_time_default_value() {
        if let TokenValue::Time(time) =
            TokenValue::get_default_value_for_header(&ParamType::Time).unwrap()
        {
            let now = Utc::now().timestamp_millis() as u64;
            assert!(time <= now && time >= now - 1000);
        } else {
            panic!("Wrong value type");
        }
    }

    #[test]
    fn test_default_values() {
        let param_types = vec![ParamType::Expire, ParamType::PublicKey];
        let default_values = vec![TokenValue::Expire(0xffffffff), TokenValue::PublicKey(None)];

        for (param_type, value) in param_types.iter().zip(default_values) {
            assert_eq!(
                TokenValue::get_default_value_for_header(&param_type).unwrap(),
                value
            );
        }
    }
}
