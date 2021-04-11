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

use ed25519::signature::{Signature, Signer};

use ton_types::{BuilderData, SliceData, IBitstring};
use ton_types::dictionary::HashmapE;
use ton_block::{MsgAddressInt, Serializable};

use json_abi::*;

const WALLET_ABI: &str = r#"{
    "ABI version": 2,
    "header": [
        "expire",
        "pubkey"
    ],
    "functions": [
        {
            "name": "sendTransaction",
            "inputs": [
                {"name":"dest","type":"address"},
                {"name":"value","type":"uint128"},
                {"name":"bounce","type":"bool"}
            ],
            "outputs": [
            ]
        },
        {
            "name": "setSubscriptionAccount",
            "inputs": [
                {"name":"addr","type":"address"}
            ],
            "outputs": [
            ]
        },
        {
            "name": "getSubscriptionAccount",
            "inputs": [
            ],
            "outputs": [
                {"name":"value0","type":"address"}
            ]
        },
        {
            "name": "createOperationLimit",
            "inputs": [
                {"name":"value","type":"uint256"}
            ],
            "outputs": [
                {"name":"value0","type":"uint256"}
            ]
        },
        {
            "name": "createArbitraryLimit",
            "inputs": [
                {"name":"value","type":"uint128"},
                {"name":"period","type":"uint32"}
            ],
            "outputs": [
                {"name":"value0","type":"uint64"}
            ]
        },
        {
            "name": "changeLimit",
            "inputs": [
                {"name":"limitId","type":"uint64"},
                {"name":"value","type":"uint256"},
                {"name":"period","type":"uint32"}
            ],
            "outputs": [
            ]
        },
        {
            "name": "deleteLimit",
            "inputs": [
                {"name":"limitId","type":"uint64"}
            ],
            "outputs": [
            ]
        },
        {
            "name": "getLimit",
            "inputs": [
                {"name":"limitId","type":"uint64"}
            ],
            "outputs": [
                {"components":[{"name":"value","type":"uint256"},{"name":"period","type":"uint32"},{"name":"ltype","type":"uint8"},{"name":"spent","type":"uint256"},{"name":"start","type":"uint32"}],"name":"value0","type":"tuple"}
            ]
        },
        {
            "name": "getLimitCount",
            "inputs": [
            ],
            "outputs": [
                {"name":"value0","type":"uint64"}
            ]
        },
        {
            "name": "getLimits",
            "inputs": [
            ],
            "outputs": [
                {"name":"value0","type":"uint64[]"}
            ]
        },
        {
            "name": "constructor",
            "inputs": [
            ],
            "outputs": [
            ]
        }
    ],
    "events": [{
        "name": "event",
        "inputs": [
            {"name":"param","type":"uint8"}
        ]
    }
    ],
    "data": [
        {"key":101,"name":"subscription","type":"address"},
        {"key":100,"name":"owner","type":"uint256"}
    ]
}
"#;

#[test]
fn test_constructor_call() {
    let params = r#"{}"#;

    let test_tree = encode_function_call(
        WALLET_ABI.to_owned(),
        "constructor".to_owned(),
        None,
        params.to_owned(),
        false,
        None,
    ).unwrap();

    let mut expected_tree = BuilderData::new();
    expected_tree.append_bit_zero().unwrap();       // None for signature
    expected_tree.append_u32(0xffffffff).unwrap();  // max u32 for expire
    expected_tree.append_bit_zero().unwrap();       // None for public key
    expected_tree.append_u32(0x68B55F3F).unwrap();  // function id

    let test_tree = SliceData::from(test_tree);
    let expected_tree = SliceData::from(expected_tree);
    assert_eq!(test_tree, expected_tree);

    let response = decode_unknown_function_call(
        WALLET_ABI.to_owned(),
        test_tree.clone(),
        false
    ).unwrap();

    assert_eq!(response.params, params);
    assert_eq!(response.function_name, "constructor");


    let test_tree = SliceData::from_raw(vec![0xE8, 0xB5, 0x5F, 0x3F], 32);

    let response = decode_unknown_function_response(
        WALLET_ABI.to_owned(),
        test_tree.clone(),
        false
    )
    .unwrap();

    assert_eq!(response.params, params);
    assert_eq!(response.function_name, "constructor");


    let response = decode_function_response(
        WALLET_ABI.to_owned(),
        "constructor".to_owned(),
        test_tree,
        false
    )
    .unwrap();

    assert_eq!(response, params);
}

#[test]
fn test_signed_call() {
    let params = r#"
    {
        "value": 12,
        "period": 30
    }"#;

    let expected_params = r#"{"value":"12","period":"30"}"#;

    let pair = Keypair::generate(&mut rand::thread_rng());

    let test_tree = encode_function_call(
        WALLET_ABI.to_owned(),
        "createArbitraryLimit".to_owned(),
        None,
        params.to_owned(),
        false,
        Some(&pair),
    )
    .unwrap();

    let mut test_tree = SliceData::from(test_tree);

    let response = decode_unknown_function_call(
        WALLET_ABI.to_owned(),
        test_tree.clone(),
        false
    )
    .unwrap();

    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.params).unwrap(),
        serde_json::from_str::<serde_json::Value>(&expected_params).unwrap());
    assert_eq!(response.function_name, "createArbitraryLimit");

    let mut expected_tree = BuilderData::new();
    expected_tree.append_u32(0xffffffff).unwrap();          // expire
    expected_tree.append_bit_one().unwrap();                // Some for public key
    expected_tree.append_raw(&pair.public.to_bytes(), ed25519_dalek::PUBLIC_KEY_LENGTH * 8).unwrap();
    expected_tree.append_u32(0x2238B58A).unwrap();          // function id
    expected_tree.append_raw(&[0; 15], 15 * 8).unwrap();    // value
    expected_tree.append_u8(12).unwrap();                   // value
    expected_tree.append_u32(30).unwrap();                  // period

    assert!(test_tree.get_next_bit().unwrap());
    let sign = &test_tree.get_next_bytes(ed25519_dalek::SIGNATURE_LENGTH).unwrap();
    let sign = Signature::from_bytes(sign).unwrap();

    assert_eq!(test_tree, SliceData::from(expected_tree));

    let hash = test_tree.into_cell().repr_hash();
    pair.verify(hash.as_slice(), &sign).unwrap();

    let expected_response = r#"{"value0":"0"}"#;

    let response_tree = SliceData::from(
        BuilderData::with_bitstring(
            vec![0xA2, 0x38, 0xB5, 0x8A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80])
        .unwrap());

    let response = decode_function_response(
        WALLET_ABI.to_owned(),
        "createArbitraryLimit".to_owned(),
        response_tree.clone(),
        false
    )
    .unwrap();

    assert_eq!(response, expected_response);


    let response = decode_unknown_function_response(
        WALLET_ABI.to_owned(),
        response_tree,
        false
    )
    .unwrap();

    assert_eq!(response.params, expected_response);
    assert_eq!(response.function_name, "createArbitraryLimit");
}

#[test]
fn test_not_signed_call() {
    let params = r#"{
        "limitId": "0x2"
    }"#;
    let header = r#"{
        "pubkey": "11c0a428b6768562df09db05326595337dbb5f8dde0e128224d4df48df760f17",
        "expire": 123
    }"#;

    let test_tree = encode_function_call(
        WALLET_ABI.to_owned(),
        "getLimit".to_owned(),
        Some(header.to_owned()),
        params.to_owned(),
        false,
        None,
    )
    .unwrap();

    let mut expected_tree = BuilderData::new();
    expected_tree.append_bit_zero().unwrap();        // None for signature
    expected_tree.append_u32(123).unwrap();          // expire
    expected_tree.append_bit_one().unwrap();         // Some for public key
    expected_tree.append_raw(
        &hex::decode("11c0a428b6768562df09db05326595337dbb5f8dde0e128224d4df48df760f17").unwrap(),
        32 * 8).unwrap();                            // pubkey
    expected_tree.append_u32(0x4B774C98).unwrap();   // function id
    expected_tree.append_u64(2).unwrap();            // limitId

    assert_eq!(test_tree, expected_tree);
}

#[test]
fn test_add_signature_full() {
    let params = r#"{"limitId":"2"}"#;
    let header = "{}";

    let (msg, data_to_sign) = prepare_function_call_for_sign(
        WALLET_ABI.to_owned(),
        "getLimit".to_owned(),
        Some(header.to_owned()),
        params.to_owned()
    )
    .unwrap();

    let pair = Keypair::generate(&mut rand::thread_rng());
    let signature = pair.sign(&data_to_sign).to_bytes().to_vec();

    let msg = add_sign_to_function_call(
        WALLET_ABI.to_owned(),
        &signature,
        Some(&pair.public.to_bytes()),
        msg.into()).unwrap();

    let decoded = decode_unknown_function_call(WALLET_ABI.to_owned(), msg.into(), false).unwrap();

    assert_eq!(decoded.params, params);
}

#[test]
fn test_find_event() {
    let event_tree = SliceData::from(
        BuilderData::with_bitstring(
            vec![0x0C, 0xAF, 0x24, 0xBE, 0xFF, 0x80])
        .unwrap());

    let decoded = decode_unknown_function_response(WALLET_ABI.to_owned(), event_tree, false).unwrap();

    assert_eq!(decoded.function_name, "event");
    assert_eq!(decoded.params, r#"{"param":"255"}"#);
}

#[test]
fn test_store_pubkey() {
    let mut test_map = HashmapE::with_bit_len(Contract::DATA_MAP_KEYLEN);
    let test_pubkey = vec![11u8; 32];
    test_map.set_builder(
        0u64.write_to_new_cell().unwrap().into(),
        &BuilderData::with_raw(vec![0u8; 32], 256).unwrap(),
    ).unwrap();

    let data = test_map.write_to_new_cell().unwrap();

    let new_data = Contract::insert_pubkey(data.into(), &test_pubkey).unwrap();

    let new_map = HashmapE::with_hashmap(Contract::DATA_MAP_KEYLEN, new_data.reference_opt(0));
    let key_slice = new_map.get(
        0u64.write_to_new_cell().unwrap().into(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(key_slice.get_bytestring(0), test_pubkey);
}

#[test]
fn test_update_contract_data() {
    let mut test_map = HashmapE::with_bit_len(Contract::DATA_MAP_KEYLEN);
    test_map.set_builder(
        0u64.write_to_new_cell().unwrap().into(),
        &BuilderData::with_raw(vec![0u8; 32], 256).unwrap(),
    ).unwrap();

    let params = r#"{
        "subscription": "0:1111111111111111111111111111111111111111111111111111111111111111",
        "owner": "0x2222222222222222222222222222222222222222222222222222222222222222"
     }
    "#;

    let data = test_map.write_to_new_cell().unwrap();
    let new_data = update_contract_data(WALLET_ABI, params, data.into()).unwrap();
    let new_map = HashmapE::with_hashmap(Contract::DATA_MAP_KEYLEN, new_data.reference_opt(0));


    let key_slice = new_map.get(
        0u64.write_to_new_cell().unwrap().into(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(key_slice.get_bytestring(0), vec![0u8; 32]);


    let subscription_slice = new_map.get(
        101u64.write_to_new_cell().unwrap().into(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        subscription_slice,
        MsgAddressInt::with_standart(None, 0, vec![0x11; 32].into()).unwrap().write_to_new_cell().unwrap().into());


    let owner_slice = new_map.get(
        100u64.write_to_new_cell().unwrap().into(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(owner_slice.get_bytestring(0), vec![0x22; 32]);
}
