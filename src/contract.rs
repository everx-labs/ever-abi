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

use crate::{
    error::AbiError, event::Event, function::Function, param::Param, 
    param_type::ParamType, token::Token
};
use std::io;
use std::collections::HashMap;
use serde::de::{Error as SerdeError};
use serde_json;
use ton_block::Serializable;
use ton_types::{BuilderData, error, fail, HashmapE, Result, SliceData};

pub const SUPPORTED_VERSIONS: [u8; 2] = [1, 2];

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
    /// ABI version.
    #[serde(rename="ABI version")]
    pub abi_version: u8,
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
}

pub struct DecodedMessage {
    pub function_name: String,
    pub tokens: Vec<Token>,
    pub params: Vec<Param>
}

/// API building calls to contracts ABI.
#[derive(Clone, Debug, PartialEq)]
pub struct Contract {
    /// ABI version
    abi_version: u8,
    /// Contract functions header parameters
    header: Vec<Param>,
    /// Contract functions.
    functions: HashMap<String, Function>,
    /// Contract events.
    events: HashMap<String, Event>,
    /// Contract initila data.
    data: HashMap<String, DataItem>,
}

impl Contract {
    /// Loads contract from json.
    pub fn load<T: io::Read>(reader: T) -> Result<Self> {
        // A little trick similar to `Param` deserialization: first deserialize JSON into temporary 
        // struct `SerdeContract` containing necessary fields and then repack fields into HashMap
        let mut serde_contract: SerdeContract = serde_json::from_reader(reader)?;
        let version = serde_contract.abi_version;

        if !SUPPORTED_VERSIONS.contains(&version) {
            fail!(AbiError::WrongVersion{ version: serde_contract.abi_version });
        }

        if version == 1 {
            if serde_contract.header.len() != 0 {
                return Err(AbiError::InvalidData {
                    msg: "Header parameters are not supported in ABI v1".into()
                }.into());
            }
            if serde_contract.set_time {
                serde_contract.header.push(Param { name: "time".into(), kind: ParamType::Time});
            }
        }

        let mut result = Self {
            abi_version: version,
            header: serde_contract.header,
            functions: HashMap::new(),
            events: HashMap::new(),
            data: HashMap::new(),
        };

        for function in serde_contract.functions {
            Self::check_params_support(version, function.inputs.iter())?;
            Self::check_params_support(version, function.outputs.iter())?;
            result.functions.insert(
                function.name.clone(),
                Function::from_serde(version, function, result.header.clone()));
        }

        for event in serde_contract.events {
            Self::check_params_support(version, event.inputs.iter())?;
            result.events.insert(event.name.clone(), Event::from_serde(version, event));
        }

        Self::check_params_support(version, serde_contract.data.iter().map(|val| &val.value))?;
        for data in serde_contract.data {
            result.data.insert(data.value.name.clone(), data);
        }

        Ok(result)
    }

    fn check_params_support<'a, T>(abi_version: u8, params: T) -> Result<()>
        where 
        T: std::iter::Iterator<Item = &'a Param>
    {
        for param in params {
            if !param.kind.is_supported(abi_version) {
                return Err(AbiError::InvalidData {
                    msg: "Header parameters are not supported in ABI v1".into()
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

    /// Decodes contract answer and returns name of the function called
    pub fn decode_output(&self, data: SliceData, internal: bool) -> Result<DecodedMessage> {
        let original_data = data.clone();
        
        let func_id = Function::decode_output_id(data)?;

        if let Ok(func) = self.function_by_id(func_id, false){
            let tokens = func.decode_output(original_data, internal)?;

            Ok( DecodedMessage {
                function_name: func.name.clone(),
                tokens: tokens,
                params: func.output_params().clone()
            })
        } else {
            let event = self.event_by_id(func_id)?;
            let tokens = event.decode_input(original_data)?;

            Ok( DecodedMessage {
                function_name: event.name.clone(),
                tokens: tokens,
                params: event.input_params()
            })
        }
    }

    /// Decodes contract answer and returns name of the function called
    pub fn decode_input(&self, data: SliceData, internal: bool) -> Result<DecodedMessage> {
        let original_data = data.clone();
        
        let func_id = Function::decode_input_id(self.abi_version, data, &self.header, internal)?;

        let func = self.function_by_id(func_id, true)?;

        let tokens = func.decode_input(original_data, internal)?;

        Ok( DecodedMessage {
            function_name: func.name.clone(),
            tokens: tokens,
            params: func.input_params().clone()
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
            let builder = token.value.pack_into_chain(self.abi_version)?;
            let key = self.data
                .get(&token.name)
                .ok_or_else(||
                    AbiError::InvalidData { msg: format!("data item {} not found in contract ABI", token.name) }
                )?.key;

                map.set(
                    key.write_to_new_cell().unwrap().into(), 
                    &builder.into(), 
                )?;
        }

        Ok(map.write_to_new_cell()?.into())
    }

    // Gets public key from contract data
    pub fn get_pubkey(data: &SliceData) -> Result<Option<Vec<u8>>> {
        let map = HashmapE::with_hashmap(
            Self::DATA_MAP_KEYLEN,
            data.reference_opt(0),
        );
        map.get(0u64.write_to_new_cell()?.into())
            .map(|opt| opt.map(|slice| slice.get_bytestring(0)))
    }

    /// Sets public key into contract data
    pub fn insert_pubkey(data: SliceData, pubkey: &[u8]) -> Result<SliceData> {
        let pubkey_vec = pubkey.to_vec();
        let pubkey_len = pubkey_vec.len() * 8;
        let value = BuilderData::with_raw(pubkey_vec, pubkey_len).unwrap_or_default();

        let mut map = HashmapE::with_hashmap(
            Self::DATA_MAP_KEYLEN, 
            data.reference_opt(0)
        );
        map.set(
            0u64.write_to_new_cell().unwrap().into(), 
            &value.into(), 
        )?;
        Ok(map.write_to_new_cell()?.into())
    }

    /// Add sign to messsage body returned by `prepare_input_for_sign` function
    pub fn add_sign_to_encoded_input(
        &self,
        signature: &[u8],
        public_key: Option<&[u8]>,
        function_call: SliceData
    ) -> Result<BuilderData> {
        Function::add_sign_to_encoded_input(self.abi_version, signature, public_key, function_call)
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