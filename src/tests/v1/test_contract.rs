/*
* Copyright 2018-2020 TON DEV SOLUTIONS LTD.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.  You may obtain a copy of the
* License at: https://ton.dev/licenses
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific TON DEV software governing permissions and
* limitations under the License.
*/

use {Contract, Function, Event, Param, ParamType, DataItem};
use std::collections::HashMap;

const TEST_ABI: &str = r#"
{
    "ABI version": 1,
    "functions": [{
            "name": "input_and_output",
            "inputs": [
                {"name": "a","type": "uint64"},
                {"name": "b","type": "uint8[]"},
                {"name": "c","type": "bytes"}
            ],
            "outputs": [
                {"name": "a","type": "int16"},
                {"name": "b","type": "uint8"}
            ]
        }, {
            "name": "no_output",
            "inputs": [{"name": "a", "type": "uint15"}],
            "outputs": []
        }, {
            "name": "no_input",
            "inputs": [],
            "outputs": [{"name": "a", "type": "uint8"}]
        }, {
            "name": "constructor",
            "inputs": [],
            "outputs": []
        },
        {
            "name": "has_id",
            "id": "0x01234567",
            "inputs": [],
            "outputs": []
        }],
    "events": [{
        "name": "input",
        "inputs": [{"name": "a","type": "uint64"}]
    }, {
        "name": "no_input",
        "inputs": []
    }, {
        "name": "has_id",
        "id": "0x89abcdef",
        "inputs": []
    }],
    "data": [
        {"key":100,"name":"a","type":"uint256"}
    ]
}"#;

#[test]
fn test_abi_parse() {
    let parsed_contract = Contract::load(TEST_ABI.as_bytes()).unwrap();

    let mut functions = HashMap::new();
    let header = vec![Param { name: "time".into(), kind: ParamType::Time}];

    functions.insert(
        "input_and_output".to_owned(),
        Function {
            abi_version: 1,
            name: "input_and_output".to_owned(),
            header: header.clone(),
            inputs: vec![
                Param { name: "a".to_owned(), kind: ParamType::Uint(64) },
                Param { name: "b".to_owned(), kind: ParamType::Array(
                    Box::new(ParamType::Uint(8))) },
                Param { name: "c".to_owned(), kind: ParamType::Bytes },
            ],
            outputs: vec![
                Param { name: "a".to_owned(), kind: ParamType::Int(16) },
                Param { name: "b".to_owned(), kind: ParamType::Uint(8) },
            ],
            input_id: Function::calc_function_id("input_and_output(time,uint64,uint8[],bytes)(int16,uint8)v1") & 0x7FFFFFFF,
            output_id: Function::calc_function_id("input_and_output(time,uint64,uint8[],bytes)(int16,uint8)v1") | 0x80000000
        });

    functions.insert(
        "no_output".to_owned(),
        Function {
            abi_version: 1,
            name: "no_output".to_owned(),
            header: header.clone(),
            inputs: vec![
                Param { name: "a".to_owned(), kind: ParamType::Uint(15) },
            ],
            outputs: vec![],
            input_id: Function::calc_function_id("no_output(time,uint15)()v1") & 0x7FFFFFFF,
            output_id: Function::calc_function_id("no_output(time,uint15)()v1") | 0x80000000
        });

    functions.insert(
        "no_input".to_owned(),
        Function {
            abi_version: 1,
            name: "no_input".to_owned(),
            header: header.clone(),
            inputs: vec![],
            outputs: vec![
                Param { name: "a".to_owned(), kind: ParamType::Uint(8) },
            ],
            input_id: Function::calc_function_id("no_input(time)(uint8)v1") & 0x7FFFFFFF,
            output_id: Function::calc_function_id("no_input(time)(uint8)v1") | 0x80000000
        });

    functions.insert(
        "constructor".to_owned(),
        Function {
            abi_version: 1,
            name: "constructor".to_owned(),
            header: header.clone(),
            inputs: vec![],
            outputs: vec![],
            input_id: Function::calc_function_id("constructor(time)()v1") & 0x7FFFFFFF,
            output_id: Function::calc_function_id("constructor(time)()v1") | 0x80000000
        });

    functions.insert(
        "has_id".to_owned(),
        Function {
            abi_version: 1,
            name: "has_id".to_owned(),
            header: header.clone(),
            inputs: vec![],
            outputs: vec![],
            input_id: 0x01234567,
            output_id: 0x01234567
        });

    let mut events = HashMap::new();

    events.insert(
        "input".to_owned(),
        Event {
            abi_version: 1,
            name: "input".to_owned(),
            inputs: vec![
                Param { name: "a".to_owned(), kind: ParamType::Uint(64) },
            ],
            id: Function::calc_function_id("input(uint64)v1") & 0x7FFFFFFF
        });

    events.insert(
        "no_input".to_owned(),
        Event {
            abi_version: 1,
            name: "no_input".to_owned(),
            inputs: vec![],
            id: Function::calc_function_id("no_input()v1") & 0x7FFFFFFF
        });

    events.insert(
        "has_id".to_owned(),
        Event {
            abi_version: 1,
            name: "has_id".to_owned(),
            inputs: vec![],
            id: 0x89abcdef
        });

    let mut data = HashMap::new();

    data.insert(
        "a".to_owned(),
        DataItem {
            value: Param {
                name: "a".to_owned(),
                kind: ParamType::Uint(256)
            },
            key: 100
        });

    let expected_contract = Contract { abi_version: 1, header, functions, events, data };

    assert_eq!(parsed_contract, expected_contract);
}

#[test]
fn print_function_singnatures() {
    let contract = Contract::load(TEST_ABI.as_bytes()).unwrap();

    println!("Functions\n");

    let functions = contract.functions();

    for (_, function) in functions {
        println!("{}", function.get_function_signature());
        let id = function.get_function_id();
        println!("{:X?}\n", id);
    }

    println!("Events\n");

    let events = contract.events();

    for (_, event) in events {
        println!("{}", event.get_function_signature());
        let id = event.get_function_id();
        println!("{:X?}\n", id);
    }
}

const TEST_ABI_WRONG_VERSION: &str = r#"
{
    "ABI version": 0,
    "functions": [{
            "name": "constructor",
            "inputs": [],
            "outputs": [],
            "signed": false
        }]
}"#;

#[test]
fn test_abi_wrong_version() {
    assert!(Contract::load(TEST_ABI_WRONG_VERSION.as_bytes()).is_err());
}

const TEST_ABI_HEADER_IN_V1: &str = r#"
{
    "ABI version": 1,
    header: ["time"],
    "functions": []
}"#;

#[test]
fn test_abi_header_in_v1() {
    assert!(Contract::load(TEST_ABI_HEADER_IN_V1.as_bytes()).is_err());
}
