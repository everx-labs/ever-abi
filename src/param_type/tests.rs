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

mod param_type_tests {
    use ParamType;
    use Param;

    #[test]
    fn test_param_type_signature() {
        assert_eq!(ParamType::Uint(256).type_signature(), "uint256".to_owned());
        assert_eq!(ParamType::Int(64).type_signature(), "int64".to_owned());
        assert_eq!(ParamType::Bool.type_signature(), "bool".to_owned());

        assert_eq!(
            ParamType::Array(Box::new(ParamType::Cell)).type_signature(),
            "cell[]".to_owned());

        assert_eq!(
            ParamType::FixedArray(Box::new(ParamType::Int(33)), 2).type_signature(),
            "int33[2]".to_owned());

        assert_eq!(
            ParamType::FixedArray(Box::new(ParamType::Array(Box::new(ParamType::Bytes))), 2)
                .type_signature(),
            "bytes[][2]".to_owned());

        let mut tuple_params = vec![];
        tuple_params.push(Param {name: "a".to_owned(), kind: ParamType::Uint(123)});
        tuple_params.push(Param {name: "b".to_owned(), kind: ParamType::Int(8)});

        let tuple_with_tuple = vec![
            Param {name: "a".to_owned(), kind: ParamType::Tuple(tuple_params.clone())},
            Param {name: "b".to_owned(), kind: ParamType::Gram}
        ];

        assert_eq!(
            ParamType::Tuple(tuple_params.clone()).type_signature(),
            "(uint123,int8)".to_owned());

        assert_eq!(
            ParamType::Array(Box::new(ParamType::Tuple(tuple_with_tuple))).type_signature(),
            "((uint123,int8),gram)[]".to_owned());

        assert_eq!(
            ParamType::FixedArray(Box::new(ParamType::Tuple(tuple_params)), 4).type_signature(),
            "(uint123,int8)[4]".to_owned());

        assert_eq!(
            ParamType::Map(Box::new(ParamType::Int(456)), Box::new(ParamType::Address)).type_signature(),
            "map(int456,address)".to_owned());

        assert_eq!(ParamType::String.type_signature(), "string".to_owned());

        assert_eq!(ParamType::VarUint(16).type_signature(), "varuint16".to_owned());
        assert_eq!(ParamType::VarInt(32).type_signature(), "varint32".to_owned());
    }
}

mod deserialize_tests {
    use serde_json;
    use ParamType;

    #[test]
    fn param_type_deserialization() {
        let s = r#"["uint256", "int64", "bool", "bool[]", "int33[2]", "bool[][2]",
            "tuple", "tuple[]", "tuple[4]", "cell", "map(int3,bool)", "map(uint1023,tuple[][5])",
            "address", "bytes", "fixedbytes32", "gram", "time", "expire", "pubkey", "string",
            "varuint16", "varint32"]"#;
        let deserialized: Vec<ParamType> = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, vec![
            ParamType::Uint(256),
            ParamType::Int(64),
            ParamType::Bool,
            ParamType::Array(Box::new(ParamType::Bool)),
            ParamType::FixedArray(Box::new(ParamType::Int(33)), 2),
            ParamType::FixedArray(Box::new(ParamType::Array(Box::new(ParamType::Bool))), 2),
            ParamType::Tuple(vec![]),
            ParamType::Array(Box::new(ParamType::Tuple(vec![]))),
            ParamType::FixedArray(Box::new(ParamType::Tuple(vec![])), 4),
            ParamType::Cell,
            ParamType::Map(Box::new(ParamType::Int(3)), Box::new(ParamType::Bool)),
            ParamType::Map(
                Box::new(ParamType::Uint(1023)),
                Box::new(ParamType::FixedArray(
                    Box::new(ParamType::Array(
                        Box::new(ParamType::Tuple(vec![])))),
                    5))),
            ParamType::Address,
            ParamType::Bytes,
            ParamType::FixedBytes(32),
            ParamType::Gram,
            ParamType::Time,
            ParamType::Expire,
            ParamType::PublicKey,
            ParamType::String,
            ParamType::VarUint(16),
            ParamType::VarInt(32),
        ]);
    }
}
