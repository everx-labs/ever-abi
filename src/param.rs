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

//! Function param.
use crate::param_type::ParamType;
use serde::de::{Deserialize, Deserializer, Error};

/// Function param.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// Param name.
    pub name: String,
    /// Param type.
    pub kind: ParamType,
}

impl Param {
    pub fn new(name: &str, kind: ParamType) -> Self {
        Self {
            name: name.to_string(),
            kind,
        }
    }

    pub(crate) fn from_serde(serde_param: SerdeParam) -> Result<Self, String> {
        let mut result = Self {
            name: serde_param.name,
            kind: serde_param.kind,
        };

        result
            .kind
            .set_components(serde_param.components)
            .map_err(|err| err.to_string())?;

        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub(crate) struct SerdeParam {
    /// Param name.
    pub name: String,
    /// Param type.
    #[serde(rename = "type")]
    pub kind: ParamType,
    /// Tuple components
    #[serde(default)]
    pub components: Vec<Param>,
    /// `init` flag for fields section
    #[serde(default)]
    pub init: bool,
}

impl<'a> Deserialize<'a> for Param {
    fn deserialize<D>(deserializer: D) -> Result<Param, D::Error>
    where
        D: Deserializer<'a>,
    {
        // A little trick: tuple parameters is described in JSON as addition field `components`
        // but struct `Param` doesn't have such a field and tuple components is stored inside of
        // `ParamType::Tuple` enum. To use automated deserialization instead of manual parameters
        // recognizing we first deserialize parameter into temp struct `SerdeParam` and then
        // if parameter is a tuple repack tuple components from `SerdeParam::components`
        // into `ParamType::Tuple`
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_string() {
            let type_str = value.as_str().unwrap();
            let param_type: ParamType =
                serde_json::from_value(value.clone()).map_err(|err| D::Error::custom(err))?;
            match param_type {
                ParamType::Tuple(_) |
                ParamType::Array(_) |
                ParamType::FixedArray(_, _) |
                ParamType::Map(_, _) =>
                    return Err(D::Error::custom(
                        format!("Invalid parameter specification: {}. Only simple types can be represented as strings",
                            type_str))),
                _ => {}
            }
            Ok(Self {
                name: type_str.to_owned(),
                kind: param_type,
            })
        } else {
            let serde_param: SerdeParam =
                serde_json::from_value(value).map_err(|err| D::Error::custom(err))?;

            Self::from_serde(serde_param).map_err(D::Error::custom)
        }
    }
}

#[cfg(test)]
#[path = "tests/test_param.rs"]
mod tests;
