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

extern crate sha2;
extern crate num_bigint;
extern crate hex;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
extern crate ton_block;
extern crate ton_types;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate ed25519;
extern crate ed25519_dalek;
extern crate base64;
extern crate chrono;
extern crate failure;
extern crate num_traits;

pub mod contract;
pub mod function;
pub mod event;
pub mod int;
pub mod param;
pub mod param_type;
pub mod token;
pub mod json_abi;
pub mod error;

pub use param_type::ParamType;
pub use contract::{Contract, DataItem};
pub use token::{Token, TokenValue};
pub use function::Function;
pub use event::Event;
pub use json_abi::*;
pub use param::Param;
pub use int::{Int, Uint};
pub use error::*;

#[cfg(test)]
extern crate rand;
extern crate byteorder;

#[cfg(test)]
pub(crate) mod abi_conv {
    use ::{TokenValue, Uint};

    fn ustr(n: u128, size: usize) -> String {
        serde_json::to_value(&TokenValue::Uint(Uint::new(n, size)))
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    #[cfg(test)]
    pub(crate) fn u8str(n: u8) -> String {
        ustr(n as u128, 8)
    }

    #[cfg(test)]
    pub(crate) fn u32str(n: u32) -> String {
        ustr(n as u128, 32)
    }

    #[cfg(test)]
    pub(crate) fn u64str(n: u64) -> String {
        ustr(n as u128, 64)
    }

    #[cfg(test)]
    pub(crate) fn u128str(n: u128) -> String {
        ustr(n, 128)
    }

    #[cfg(test)]
    pub(crate) fn u256str(n: u128) -> String {
        ustr(n, 256)
    }

    #[cfg(test)]
    pub(crate) fn i16str(n: i16) -> String {
        n.to_string()
    }
}
