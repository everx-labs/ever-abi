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

const WALLET_ABI_V23: &str = r#"{
    "version": "2.3",
    "header": [
        "expire",
        "pubkey"
    ],
    "functions": [
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
            "name": "getLimit",
            "inputs": [
                {"name":"limitId","type":"uint64"}
            ],
            "outputs": [
                {"components":[{"name":"value","type":"uint256"},{"name":"period","type":"uint32"},{"name":"ltype","type":"uint8"},{"name":"spent","type":"uint256"},{"name":"start","type":"uint32"}],"name":"value0","type":"tuple"}
            ]
        }
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
        None,
    ).unwrap();

    let mut expected_tree = BuilderData::new();
    expected_tree.append_bit_zero().unwrap();       // None for signature
    expected_tree.append_u32(0xffffffff).unwrap();  // max u32 for expire
    expected_tree.append_bit_zero().unwrap();       // None for public key
    expected_tree.append_u32(0x68B55F3F).unwrap();  // function id

    let test_tree = SliceData::load_builder(test_tree).unwrap();
    let expected_tree = SliceData::load_builder(expected_tree).unwrap();
    assert_eq!(test_tree, expected_tree);

    let response = decode_unknown_function_call(
        WALLET_ABI.to_owned(),
        test_tree.clone(),
        false,
        false,
    ).unwrap();

    assert_eq!(response.params, params);
    assert_eq!(response.function_name, "constructor");


    let test_tree = SliceData::from_raw(vec![0xE8, 0xB5, 0x5F, 0x3F], 32);

    let response = decode_unknown_function_response(
        WALLET_ABI.to_owned(),
        test_tree.clone(),
        false,
        false,
    )
    .unwrap();

    assert_eq!(response.params, params);
    assert_eq!(response.function_name, "constructor");


    let response = decode_function_response(
        WALLET_ABI.to_owned(),
        "constructor".to_owned(),
        test_tree,
        false,
        false,
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
        None,
    )
    .unwrap();

    let mut test_tree = SliceData::load_builder(test_tree).unwrap();

    let response = decode_unknown_function_call(
        WALLET_ABI.to_owned(),
        test_tree.clone(),
        false,
        false,
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

    assert_eq!(test_tree, SliceData::load_builder(expected_tree).unwrap());

    let hash = test_tree.into_cell().repr_hash();
    pair.verify(hash.as_slice(), &sign).unwrap();

    let expected_response = r#"{"value0":"0"}"#;

    let response_tree = SliceData::load_builder(
        BuilderData::with_bitstring(
            vec![0xA2, 0x38, 0xB5, 0x8A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80]
        ).unwrap()
    ).unwrap();

    let response = decode_function_response(
        WALLET_ABI.to_owned(),
        "createArbitraryLimit".to_owned(),
        response_tree.clone(),
        false,
        false,
    )
    .unwrap();

    assert_eq!(response, expected_response);


    let response = decode_unknown_function_response(
        WALLET_ABI.to_owned(),
        response_tree,
        false,
        false,
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

    let test_tree_v23 = encode_function_call(
        WALLET_ABI_V23.to_owned(),
        "getLimit".to_owned(),
        Some(header.to_owned()),
        params.to_owned(),
        false,
        None,
        None,
    )
    .unwrap();
    assert_eq!(test_tree_v23, expected_tree);
}

#[test]
fn test_add_signature_full() {
    let params = r#"{"limitId":"2"}"#;
    let header = "{}";

    let (msg, data_to_sign) = prepare_function_call_for_sign(
        WALLET_ABI.to_owned(),
        "getLimit".to_owned(),
        Some(header.to_owned()),
        params.to_owned(),
        None,
    )
    .unwrap();

    let pair = Keypair::generate(&mut rand::thread_rng());
    let signature = pair.sign(&data_to_sign).to_bytes().to_vec();

    let msg = SliceData::load_builder(msg).unwrap();
    let msg = add_sign_to_function_call(
        WALLET_ABI.to_owned(),
        &signature,
        Some(&pair.public.to_bytes()),
        msg).unwrap();

    let msg = SliceData::load_builder(msg).unwrap();
    let decoded = decode_unknown_function_call(
        WALLET_ABI.to_owned(), msg, false, false,
    ).unwrap();

    assert_eq!(decoded.params, params);
}

#[test]
fn test_find_event() {
    let event_tree = SliceData::load_builder(
        BuilderData::with_bitstring(
            vec![0x0C, 0xAF, 0x24, 0xBE, 0xFF, 0x80]
        ).unwrap()
    ).unwrap();

    let decoded = decode_unknown_function_response(
        WALLET_ABI.to_owned(), event_tree, false, false,
    ).unwrap();

    assert_eq!(decoded.function_name, "event");
    assert_eq!(decoded.params, r#"{"param":"255"}"#);
}

#[test]
fn test_store_pubkey() {
    let mut test_map = HashmapE::with_bit_len(Contract::DATA_MAP_KEYLEN);
    let test_pubkey = vec![11u8; 32];
    test_map.set_builder(
        SliceData::load_builder(0u64.write_to_new_cell().unwrap()).unwrap(),
        &BuilderData::with_raw(vec![0u8; 32], 256).unwrap(),
    ).unwrap();

    let data = SliceData::load_cell(test_map.serialize().unwrap()).unwrap();

    let new_data = Contract::insert_pubkey(data.into(), &test_pubkey).unwrap();

    let new_map = HashmapE::with_hashmap(Contract::DATA_MAP_KEYLEN, new_data.reference_opt(0));
    let key_slice = new_map.get(
        SliceData::load_builder(0u64.write_to_new_cell().unwrap()).unwrap(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(key_slice.get_bytestring(0), test_pubkey);
}

#[test]
fn test_update_decode_contract_data() {
    let mut test_map = HashmapE::with_bit_len(Contract::DATA_MAP_KEYLEN);
    test_map.set_builder(
        SliceData::load_builder(0u64.write_to_new_cell().unwrap()).unwrap(),
        &BuilderData::with_raw(vec![0u8; 32], 256).unwrap(),
    ).unwrap();

    let params = r#"{
        "subscription": "0:1111111111111111111111111111111111111111111111111111111111111111",
        "owner": "0x2222222222222222222222222222222222222222222222222222222222222222"
     }
    "#;

    let data = SliceData::load_cell(test_map.serialize().unwrap()).unwrap();
    let new_data = update_contract_data(WALLET_ABI, params, data).unwrap();
    let new_map = HashmapE::with_hashmap(Contract::DATA_MAP_KEYLEN, new_data.reference_opt(0));


    let key_slice = new_map.get(
        SliceData::load_builder(0u64.write_to_new_cell().unwrap()).unwrap(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(key_slice.get_bytestring(0), vec![0u8; 32]);


    let subscription_slice = new_map.get(
        SliceData::load_builder(101u64.write_to_new_cell().unwrap()).unwrap(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        subscription_slice,
        SliceData::load_cell(MsgAddressInt::with_standart(None, 0, [0x11; 32].into()).unwrap().serialize().unwrap()).unwrap());


    let owner_slice = new_map.get(
        SliceData::load_builder(100u64.write_to_new_cell().unwrap()).unwrap(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(owner_slice.get_bytestring(0), vec![0x22; 32]);

    let decoded = decode_contract_data(WALLET_ABI, new_data, false).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(params).unwrap(),
        serde_json::from_str::<Value>(&decoded).unwrap()
    );
}

const ABI_WITH_FIELDS: &str = r#"{
    "version": "2.1",
    "functions": [],
    "fields": [
        {"name":"__pubkey","type":"uint256"},
        {"name":"__timestamp","type":"uint64"},
        {"name":"ok","type":"bool"},
        {"name":"value","type":"uint32"}
    ]
}"#;

#[test]
fn test_decode_storage_fields() {
    let mut storage = BuilderData::new();
    storage.append_bitstring(&[vec![0x55; 32], vec![0x80]].join(&[][..])).unwrap();
    storage.append_u64(123).unwrap();
    storage.append_bit_one().unwrap();
    storage.append_u32(456).unwrap();
    let storage = SliceData::load_builder(storage).unwrap();

    let decoded = decode_storage_fields(ABI_WITH_FIELDS, storage, false).unwrap();

    assert_eq!(decoded, serde_json::json!({
        "__pubkey": format!("0x{}", hex::encode([0x55; 32])),
        "__timestamp":"123",
        "ok": true,
        "value": "456"
    }).to_string());
}

#[test]
fn test_add_signature_full_v23() {
    let params = r#"{"limitId":"2"}"#;
    let header = "{}";

    let (msg, data_to_sign) = prepare_function_call_for_sign(
        WALLET_ABI_V23.to_owned(),
        "getLimit".to_owned(),
        Some(header.to_owned()),
        params.to_owned(),
        Some("0:5555555555555555555555555555555555555555555555555555555555555555".to_owned()),
    )
    .unwrap();

    let pair = Keypair::generate(&mut rand::thread_rng());
    let signature = pair.sign(&data_to_sign).to_bytes().to_vec();

    let msg = SliceData::load_builder(msg).unwrap();
    let msg = add_sign_to_function_call(
        WALLET_ABI_V23.to_owned(),
        &signature,
        Some(&pair.public.to_bytes()),
        msg).unwrap();
    let msg = SliceData::load_builder(msg).unwrap();

    let decoded = decode_unknown_function_call(
        WALLET_ABI_V23.to_owned(), msg, false, false,
    ).unwrap();

    assert_eq!(decoded.params, params);
}


#[test]
fn test_signed_call_v23() {
    let params = r#"
    {
        "value": 12,
        "period": 30
    }"#;

    let expected_params = r#"{"value":"12","period":"30"}"#;

    let pair = Keypair::generate(&mut rand::thread_rng());
    let address = "0:5555555555555555555555555555555555555555555555555555555555555555";

    let test_tree = encode_function_call(
        WALLET_ABI_V23.to_owned(),
        "createArbitraryLimit".to_owned(),
        None,
        params.to_owned(),
        false,
        Some(&pair),
        Some(address.to_owned()),
    )
    .unwrap();

    let mut test_tree = SliceData::load_builder(test_tree).unwrap();

    let response = decode_unknown_function_call(
        WALLET_ABI_V23.to_owned(),
        test_tree.clone(),
        false,
        false,
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

    let mut expected_tree_child = BuilderData::new();
    expected_tree_child.append_raw(&[0; 15], 15 * 8).unwrap();    // value
    expected_tree_child.append_u8(12).unwrap();                   // value
    expected_tree_child.append_u32(30).unwrap();                  // period

    expected_tree.checked_append_reference(expected_tree_child.into_cell().unwrap()).unwrap();

    assert!(test_tree.get_next_bit().unwrap());
    let sign = &test_tree.get_next_bytes(ed25519_dalek::SIGNATURE_LENGTH).unwrap();
    let sign = Signature::from_bytes(sign).unwrap();

    assert_eq!(test_tree, SliceData::load_builder(expected_tree).unwrap());

    let mut signed_tree = MsgAddressInt::from_str(address).unwrap().write_to_new_cell().unwrap();
    signed_tree.append_builder(&BuilderData::from_slice(&test_tree)).unwrap();

    let hash = signed_tree.into_cell().unwrap().repr_hash();
    pair.verify(hash.as_slice(), &sign).unwrap();

    let expected_response = r#"{"value0":"0"}"#;

    let response_tree = SliceData::load_builder(
        BuilderData::with_bitstring(
            vec![0xA2, 0x38, 0xB5, 0x8A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80]
        ).unwrap()
    ).unwrap();

    let response = decode_function_response(
        WALLET_ABI_V23.to_owned(),
        "createArbitraryLimit".to_owned(),
        response_tree.clone(),
        false,
        false,
    )
    .unwrap();

    assert_eq!(response, expected_response);


    let response = decode_unknown_function_response(
        WALLET_ABI_V23.to_owned(),
        response_tree,
        false,
        false,
    )
    .unwrap();

    assert_eq!(response.params, expected_response);
    assert_eq!(response.function_name, "createArbitraryLimit");
}
