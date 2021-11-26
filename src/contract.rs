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

use crate::{TokenValue, error::AbiError, event::Event, function::Function, param::Param, param_type::ParamType, token::Token};
use std::fmt::Display;
use std::io;
use std::collections::HashMap;
use serde::de::{Error as SerdeError};
use serde_json;
use ton_block::Serializable;
use ton_types::{BuilderData, error, fail, HashmapE, Result, SliceData};


pub const MIN_SUPPORTED_VERSION: AbiVersion = ABI_VERSION_1_0;
pub const MAX_SUPPORTED_VERSION: AbiVersion = ABI_VERSION_2_2;

pub const ABI_VERSION_1_0: AbiVersion = AbiVersion::from_parts(1, 0);
pub const ABI_VERSION_2_0: AbiVersion = AbiVersion::from_parts(2, 0);
pub const ABI_VERSION_2_1: AbiVersion = AbiVersion::from_parts(2, 1);
pub const ABI_VERSION_2_2: AbiVersion = AbiVersion::from_parts(2, 2);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct AbiVersion {
    pub major: u8,
    pub minor: u8,
}

impl AbiVersion {
    pub fn parse(str_version: &str) -> Result<Self> {
        let parts: Vec<&str> = str_version.split(".").collect();
        if parts.len() < 2 {
            fail!(AbiError::InvalidVersion(format!("version must consist of two parts divided by `.` ({})", str_version)));
        }

        let major = u8::from_str_radix(parts[0], 10)
            .map_err(|err| error!(AbiError::InvalidVersion(format!("can not parse version string: {} ({})", err, str_version))))?;
        let minor = u8::from_str_radix(parts[1], 10)
            .map_err(|err| error!(AbiError::InvalidVersion(format!("can not parse version string: {} ({})", err, str_version))))?;

        Ok(Self { major, minor })
    }

    pub const fn from_parts(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    pub fn is_supported(&self) -> bool {
        self >= &MIN_SUPPORTED_VERSION && self <= &MAX_SUPPORTED_VERSION
    }
}

impl Display for AbiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl From<u8> for AbiVersion {
    fn from(value: u8) -> Self {
        Self { major: value, minor: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DataItem {
    pub key: u64,
    #[serde(flatten)]
    pub value: Param,
}

struct StringVisitor;

impl<'de> serde::de::Visitor<'de> for StringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("String")
    }

    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E> where E: serde::de::Error {
        Ok(v)
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E> where E: serde::de::Error {
        Ok(v.to_string())
    }
}

pub fn deserialize_opt_u32_from_string<'de, D>(d: D) -> std::result::Result<Option<u32>, D::Error>
    where D: serde::Deserializer<'de>
{
    match d.deserialize_string(StringVisitor) {
        Err(_) => Ok(None),
        Ok(string) => {
            if !string.starts_with("0x") {
                return Err(D::Error::custom(format!("Number parsing error: number must be prefixed with 0x ({})", string)));
            }
        
            u32::from_str_radix(&string[2..], 16)
                .map_err(|err| D::Error::custom(format!("Error parsing number: {}", err)))
                .map(|value| Some(value))
        }
    }
}

/// Contract function specification.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct SerdeFunction {
    /// Function name.
    pub name: String,
    /// Function input.
    #[serde(default)]
    pub inputs: Vec<Param>,
    /// Function output.
    #[serde(default)]
    pub outputs: Vec<Param>,
    /// Calculated function ID
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_opt_u32_from_string")]
    pub id: Option<u32>
}

/// Contract event specification.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct SerdeEvent {
    /// Event name.
    pub name: String,
    /// Event input.
    #[serde(default)]
    pub inputs: Vec<Param>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_opt_u32_from_string")]
    pub id: Option<u32>
}

fn bool_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct SerdeContract {
    /// ABI version up to 2.
    #[serde(rename="ABI version")]
    pub abi_version: Option<u8>,
    /// ABI version.
    pub version: Option<String>,
    /// Set timestamp in message.
    #[serde(rename="setTime")]
    #[serde(default="bool_true")]
    pub set_time: bool,
    /// Header parameters.
    #[serde(default)]
    pub header: Vec<Param>,
    /// Contract functions.
    pub functions: Vec<SerdeFunction>,
    /// Contract events.
    #[serde(default)]
    pub events: Vec<SerdeEvent>,
    /// Contract initial data.
    #[serde(default)]
    pub data: Vec<DataItem>,
    /// Contract storage fields.
    #[serde(default)]
    pub fields: Vec<Param>,
}

pub struct DecodedMessage {
    pub function_name: String,
    pub tokens: Vec<Token>,
}

/// API building calls to contracts ABI.
#[derive(Clone, Debug, PartialEq)]
pub struct Contract {
    /// ABI version
    abi_version: AbiVersion,
    /// Contract functions header parameters
    header: Vec<Param>,
    /// Contract functions.
    functions: HashMap<String, Function>,
    /// Contract events.
    events: HashMap<String, Event>,
    /// Contract initial data.
    data: HashMap<String, DataItem>,
    /// Contract storage fields.
    fields: Vec<Param>,
}

impl Contract {
    /// Loads contract from json.
    pub fn load<T: io::Read>(reader: T) -> Result<Self> {
        // A little trick similar to `Param` deserialization: first deserialize JSON into temporary 
        // struct `SerdeContract` containing necessary fields and then repack fields into HashMap
        let mut serde_contract: SerdeContract = serde_json::from_reader(reader)?;

        let version = if let Some(str_version) = &serde_contract.version {
            AbiVersion::parse(str_version)?
        } else if let Some(version) = serde_contract.abi_version {
            AbiVersion::from_parts(version, 0)
        } else {
            fail!(AbiError::InvalidVersion("No version in ABI JSON".to_owned()));
        };

        if !version.is_supported() {
            fail!(AbiError::InvalidVersion(format!("Provided ABI version is not supported ({})", version)));
        }

        if version.major == 1 {
            if serde_contract.header.len() != 0 {
                return Err(AbiError::InvalidData {
                    msg: "Header parameters are not supported in ABI v1".into()
                }.into());
            }
            if serde_contract.set_time {
                serde_contract.header.push(Param { name: "time".into(), kind: ParamType::Time});
            }
        }

        if !serde_contract.fields.is_empty() && version < ABI_VERSION_2_1 {
            fail!(AbiError::InvalidData {msg: "Storage fields are supported since ABI v2.1".into()});
        }

        let mut result = Self {
            abi_version: version.clone(),
            header: serde_contract.header,
            functions: HashMap::new(),
            events: HashMap::new(),
            data: HashMap::new(),
            fields: serde_contract.fields,
        };

        for function in serde_contract.functions {
            Self::check_params_support(&version, function.inputs.iter())?;
            Self::check_params_support(&version, function.outputs.iter())?;
            result.functions.insert(
                function.name.clone(),
                Function::from_serde(version.clone(), function, result.header.clone()));
        }

        for event in serde_contract.events {
            Self::check_params_support(&version, event.inputs.iter())?;
            result.events.insert(event.name.clone(), Event::from_serde(version.clone(), event));
        }

        Self::check_params_support(&version, serde_contract.data.iter().map(|val| &val.value))?;
        for data in serde_contract.data {
            result.data.insert(data.value.name.clone(), data);
        }

        Ok(result)
    }

    fn check_params_support<'a, T>(abi_version: &AbiVersion, params: T) -> Result<()>
        where 
        T: std::iter::Iterator<Item = &'a Param>
    {
        for param in params {
            if !param.kind.is_supported(abi_version) {
                return Err(AbiError::InvalidData {
                    msg: format!("Parameters of type {} are not supported in ABI v{}", param.kind, abi_version)
                }.into());
            }
        }
        Ok(())
    }

    /// Returns `Function` struct with provided function name.
    pub fn function(&self, name: &str) -> Result<&Function> {
        self.functions.get(name).ok_or_else(|| AbiError::InvalidName { name: name.to_owned() }.into())
    }

    /// Returns `Function` struct with provided function id.
    pub fn function_by_id(&self, id: u32, input: bool) -> Result<&Function> {
        for (_, func) in &self.functions {
            let func_id = if input { func.get_input_id() } else { func.get_output_id() };
            if func_id == id {
                return Ok(func);
            }
        }

       Err(AbiError::InvalidFunctionId { id }.into())
    }

    /// Returns `Event` struct with provided function id.
    pub fn event_by_id(&self, id: u32) -> Result<&Event> {
        for (_, event) in &self.events {
            if event.get_id() == id {
                return Ok(event);
            }
        }

        Err(AbiError::InvalidFunctionId { id }.into())
    }

    /// Returns functions collection
    pub fn functions(&self) -> &HashMap<String, Function> {
        &self.functions
    }

    /// Returns header parameters set
    pub fn header(&self) -> &Vec<Param> {
        &self.header
    }

    /// Returns events collection
    pub fn events(&self) -> &HashMap<String, Event> {
        &self.events
    }

    /// Returns data collection
    pub fn data(&self) -> &HashMap<String, DataItem> {
        &self.data
    }

    /// Returns storage fields collection
    pub fn fields(&self) -> &Vec<Param> {
        &self.fields
    }

    /// Returns version
    pub fn version(&self) -> &AbiVersion {
        &self.abi_version
    }

    /// Decodes contract answer and returns name of the function called
    pub fn decode_output(&self, data: SliceData, internal: bool) -> Result<DecodedMessage> {
        let original_data = data.clone();
        
        let func_id = Function::decode_output_id(data)?;

        if let Ok(func) = self.function_by_id(func_id, false){
            let tokens = func.decode_output(original_data, internal)?;

            Ok( DecodedMessage {
                function_name: func.name.clone(),
                tokens: tokens,
            })
        } else {
            let event = self.event_by_id(func_id)?;
            let tokens = event.decode_input(original_data)?;

            Ok( DecodedMessage {
                function_name: event.name.clone(),
                tokens: tokens,
            })
        }
    }

    /// Decodes contract answer and returns name of the function called
    pub fn decode_input(&self, data: SliceData, internal: bool) -> Result<DecodedMessage> {
        let original_data = data.clone();
        
        let func_id = Function::decode_input_id(&self.abi_version, data, &self.header, internal)?;

        let func = self.function_by_id(func_id, true)?;

        let tokens = func.decode_input(original_data, internal)?;

        Ok( DecodedMessage {
            function_name: func.name.clone(),
            tokens: tokens,
        })
    }

    pub const DATA_MAP_KEYLEN: usize = 64;

    /// Changes initial values for public contract variables
    pub fn update_data(&self, data: SliceData, tokens: &[Token]) -> Result<SliceData> {
        let mut map = HashmapE::with_hashmap(
            Self::DATA_MAP_KEYLEN, 
            data.reference_opt(0),
        );

        for token in tokens {
            let builder = token.value.pack_into_chain(&self.abi_version)?;
            let key = self.data
                .get(&token.name)
                .ok_or_else(||
                    AbiError::InvalidData { msg: format!("data item {} not found in contract ABI", token.name) }
                )?.key;

                map.set_builder(
                    key.serialize()?.into(),
                    &builder, 
                )?;
        }

        Ok(map.serialize()?.into())
    }

    /// Decode initial values of public contract variables
    pub fn decode_data(&self, data: SliceData) -> Result<Vec<Token>> {
        let map = HashmapE::with_hashmap(
            Self::DATA_MAP_KEYLEN, 
            data.reference_opt(0),
        );

        let mut tokens = vec![];
        for (_, item) in &self.data {
            if let Some(value) = map.get(item.key.serialize()?.into())? {
                tokens.append(
                    &mut TokenValue::decode_params(&vec![item.value.clone()], value, &self.abi_version, false)?
                );
            }
        }

        Ok(tokens)
    }

    // Gets public key from contract data
    pub fn get_pubkey(data: &SliceData) -> Result<Option<Vec<u8>>> {
        let map = HashmapE::with_hashmap(
            Self::DATA_MAP_KEYLEN,
            data.reference_opt(0),
        );
        map.get(0u64.serialize()?.into())
            .map(|opt| opt.map(|slice| slice.get_bytestring(0)))
    }

    /// Sets public key into contract data
    pub fn insert_pubkey(data: SliceData, pubkey: &[u8]) -> Result<SliceData> {
        let pubkey_vec = pubkey.to_vec();
        let pubkey_len = pubkey_vec.len() * 8;
        let value = BuilderData::with_raw(pubkey_vec, pubkey_len)?;

        let mut map = HashmapE::with_hashmap(
            Self::DATA_MAP_KEYLEN, 
            data.reference_opt(0)
        );
        map.set_builder(
            0u64.serialize()?.into(),
            &value, 
        )?;
        Ok(map.serialize()?.into())
    }

    /// Add sign to messsage body returned by `prepare_input_for_sign` function
    pub fn add_sign_to_encoded_input(
        &self,
        signature: &[u8],
        public_key: Option<&[u8]>,
        function_call: SliceData
    ) -> Result<BuilderData> {
        Function::add_sign_to_encoded_input(&self.abi_version, signature, public_key, function_call)
    }

    /// Decode account storage fields
    pub fn decode_storage_fields(&self, data: SliceData) -> Result<Vec<Token>> {
        TokenValue::decode_params(&self.fields, data, &self.abi_version, false)
    }
}

#[cfg(test)]
#[path = "tests/test_contract.rs"]
mod tests_common;
#[cfg(test)]
#[path = "tests/v1/test_contract.rs"]
mod tests_v1;
#[cfg(test)]
#[path = "tests/v2/test_contract.rs"]
mod tests_v2;