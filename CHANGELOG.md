# Release Notes

All notable changes to this project will be documented in this file.

## Version 2.4

- Param in fields section extended with `init: boolean`
- `fixedbytes` – type is deprecated
- `ref(T)` – new type added

- Default values for parameter types:
- - `int<N>` – `N` zero bits.
- - `uint<N>` – `N` zero bits.
- - `varint<N>`/`varuint<N>` – `x` zero bits, where `x = [log2(N)]`.
- - `bool` – equivalent to [`int<N>`](#uintn), where `N = 1`.
- - `tuple(T1, T2, ..., Tn)` – default values for each type, i.e. `D(tuple(T1, T2, ..., Tn)) = tuple(D(T1), D(T2), ..., D(Tn))`, where `D` is defined as a function that takes ABI type and returns the corresponding default value.
- - `map(K,V)` – 1 zero bit, i.e. `b{0}`.
- - `cell` – reference to an empty cell, i.e. `^EmptyCell`.
- - `address` – `addr_none$00` constructor, i.e. 2 zero bits.
- - `bytes` – reference to an empty cell, i.e. `^EmptyCell`.
- - `string` – reference to an empty cell, i.e. `^EmptyCell`.
- - `optional(T)` – 1 zero bit, i.e. `b{0}`.
- - `T[]` – `x{00000000} b{0}`, i.e. 33 zero bits.
- - `T[k]` – encoded as an array with `k` default values of type `T`
- - `ref(T)` – reference to a cell, cell is encoded as the default value of type `T`.

## Version 2.3.130

- ABI v2.4 implemented

## Version 2.3.130

- Revert tests

## Version 2.3.124

- Return tagged dependencies and tests

## Version 2.3.83

- Fixed max integer values serialization

## Version 2.3.77

- Supported ever-types version 2.0

## Version 2.3.76

- Fix zero varint encoding
- Increase version number
- Update CHANGELOG.md file

## Version: 2.3.51

### New
 - Fix for support internal crates

## Version: 2.3.36

### New
 - Fix for support internal crates
 - Bump versions of external crates

## Version: 2.3.2

### New
 - Automatic update project. #none


## Version 2.3 - 2022-07-07

### New

- New method to calculate external inbound [message body signature](docs/ABI.md#signing-algorithm) introduced. It is now based on the destination address, as well as all previously used parameters.

    This prevents a problem where a message to one of several contracts with identical public keys and function signatures may be duplicated and sent to any other of this set of contracts and be successful.

    > This functionality is supported staring with [0.64.0](https://github.com/tonlabs/TON-Solidity-Compiler/blob/master/Changelog_TON.md#0640-2022-08-18) version of the Solidity compiler.


## Version 2.2 - 2021-07-19

### New

- [Fixed message body layout](docs/ABI.md#encoding-of-function-id-and-its-arguments) introduced in order to reduce gas consumption while parsing parameters.

    Each type gets max bit and max ref size, making message structure more predictable.


## Version 2.1 - 2021-07-19

### New

- New section [`Fields`](docs/ABI.md#fields) introduced.

    It describes internal structure of the smart contracts data as a list of variables' names with corresponding data types. It includes contract state variables and some internal contract specific hidden variables. They are listed in the order in which they are stored in the data field of the contract.

- New types introduced:
  - [`varint`](docs/ABI.md#varintn)
  - [`varuint`](docs/ABI.md#varuintn)
  - [`string`](docs/ABI.md#string) 
  - [`optional(innerType)`](docs/ABI.md#optionalinnertype)


## Version 2.0


- New [`header`](docs/ABI.md#header) JSON ABI section introduced. It contains additional parameters that are part of security measures for contracts:
  - [`time`](docs/ABI.md#time)
  - [`expire`](docs/ABI.md#expire)
  - [`pubkey`](docs/ABI.md#pubkey) (moved into header section)

- Signature moved to the root cell.
- Get methods placed in a separate section.
- The last cell reference can now be used by parameter serialization, which needs reference (cell, bytes, map, array types) if all the following parameters can fit into the current cell.


## Version 1

- Array types encoding redesigned to minimize gas consumption by contracts for encoding/decoding operations and contract code size.
- New TVM blockchain-specific types introduced:
  - [`map(K,V)`](docs/ABI.md#mapkeytypevaluetype)
  - [`address`](docs/ABI.md#address)
  - [`cell`](docs/ABI.md#cell)


## Version 0

Initial design of Application Binary Interface for TVM blockchain:

- [Message body structure](docs/ABI.md#message-body)
- [Function signature concept](docs/ABI.md#function-signature-function-id)
- Basic [types](docs/ABI.md#types-reference) and the rules of their encoding
- Cell overflow handling
- [JSON interface](docs/ABI.md#abi-json) sturcture
