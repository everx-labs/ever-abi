# TON Smart Contracts ABI 2.1 Specification

## ABI JSON

This section describes schema of the smart contracts ABI represented in JSON format.

Full ABI schema in TypeScript notation:

```ts
type Abi = {
    version: string,
    setTime?: boolean,
    header?: Param[],
    functions: Function[],
    events?: Event[],
    data?: Data[],
    fields?: Param[],
}

type Function = {
    name: string,
    inputs?: Param[],
    outputs?: Param[],
    id?: number,
};

type Event = {
    name: string,
    inputs?: Param[],
    id?: number,
};

type Data = Param & {
    key: number,
};

type Param = {
    name: string,
    type: string,
    components?: Param[],
};


```

Where:

- `version` contains string and uses semver semantics. Current version is "2.1".
- `functions` describes all functions the smart contract can handle.
- `events` describes all external outbound messages (events) produces by smart contract.
- `data` describes Hashmap with public data of the smart contract.
- `fields` describes internal structure of the smart contracts data.

## Functions
This section stays the same as in ABI 2.0

## Events
This section stays the same as in ABI 2.0

## Data
This section stays the same as in ABI 2.0

## Fields
This is a new section introduced in ABI 2.1. It describes internal structure of the smart contracts data.
This section helps to decode contract data with TON-SDK function [decode_account_data](https://github.com/tonlabs/TON-SDK/blob/master/docs/mod_abi.md#decode_account_data)

## Types

### `bool`

Boolean type.

Usage|Value|Examples
---|---|---
Cell|1 bit, `0` or `1`|
JSON|`true`, `false`|
JSON (accepts)|`true`, `false`, `0`, `1`, `"true"`, `"false"`|`0`, `true`, `"false"`

### `tuple`

Struct type, consists of fields of different types. All fields should be specified as an array in
the `components` section of the type.
For example, for structure `S`:
```Solidity
struct S {
    uint32 a;
    uint128 b;
    uint64 c;
}
```
parameter `s` of type `S` would be described like:
`{"components":[{"name":"a","type":"uint32"},{"name":"b","type":"uint128"},{"name":"c","type":"uint64"}],"name":"s","type":"tuple"}`

Usage|Value|Examples
---|---|---
Cell|chain of cells with tuple data types encoded consistently<br/>(without splitting value between cells)|
JSON|dictionary of struct field names with their values |`{"a": 1, "b": 2, "c": 3}`
JSON (accepts)|mapping of struct field names with their values |`{"a": 1, "b": 2, "c": 3}`

### `int<N>`

Fixed-sized signed integer, where `N` is a decimal bit length. Examples: `int8`, `int32`, `int256`.

Usage|Value|Examples
---|---|---
Cell|N bit, big endian|
JSON|string with hex representation|`0x12`
JSON (accepts)|number or string with decimal or hexadecimal representation|`12`, `0x10`, `"100"`

### `uint<N>`

Fixed-sized unsigned integer, where N is a decimal bit length e.g., `uint8`, `uint32`, `uint256`.
Processed like `int<N>`.

### `varint<N>`

*New type introduced in 2.1 version.*

Variable-length signed integer with bit length equal to `8 * N`, where `N`is equal to 16 or 32, e.g. `varint16`, `varint32`.

Usage|Value|Examples
---|---|---
Cell|4 (N=16) of 5 (N=32) bits that encode byte length of the number `len`<br/>followed by `len * 8` bit number in big endian|
JSON|string with hex representation|`0x12`
JSON (accepts)|number or string with decimal or hexadecimal representation|`12`, `0x10`, `"100"`

### `varuint<N>`

*New type introduced in 2.1 version.*

Variable-length unsigned integer with bit length equal to `8 * N`, where `N`is equal to 16 or 32 e.g., `varint16`, `varint32`.
Processed like `varint<N>`.

### `map(<keyType>,<valueType>)`

Hashtable mapping keys of `keyType` to values of the `valueType`, e.g., `map(int32, address)`.

Usage|Value|Examples
---|---|---
Cell|1 bit (`0` - for empty mapping, otherwise `1`) and ref to the cell with dictionary|
JSON|dictionary of keys and values|`{"0x1":"0x2"}`
JSON (accepts)|dictionary of keys and values|`{"0x1":"0x2"}`, `{"2":"3","3":"55"}`

### `cell`

TVM Cell type.

Usage|Value|Examples
---|---|---
Cell|stored in a ref|
JSON|binary hex data in base64|`"te6ccgEBAQEAEgAAH/////////////////////g="`
JSON (accepts)|binary hex data in base64|`"te6ccgEBAQEAAgAAAA=="`

### `address`

Contract address type `address`, consists of two parts: workchain id (wid) and address value.

Usage|Value|Examples
---|---|---
Cell|2 bits of address type, 1 bit of anycast, wid - 8 bit signed integer and address </br> value - 256 bit unsigned integer|
JSON|decimal signed integer and unsigned hexadecimal integer with leading zeros </br>separated by `:`| `"123:000000000000000000000000000000000000000000000000000000000001e0f3"`
JSON (accepts)|decimal signed integer and unsigned hexadecimal integer with leading zeros </br> separated by `:`| `"123:000000000000000000000000000000000000000000000000000000000001e0f3"`

### `bytes`

Byte string of data.

Usage|Value|Examples
---|---|---
Cell|cell with data stored in a ref|
JSON|binary hex data|`"313233"`
JSON (accepts)|binary hex data|`"323334"`

### `fixedbytes<N>`

Where N is a decimal byte length from 1 to 32. It is denoted in abi as `uint<M>`,
where `M` is a bit length and `M = 8 * N`.
Processed like `int<N>`.

### `string`

New type introduced in 2.1 version.

String data.

Usage|Value|Examples
---|---|---
Cell|cell with data stored in a ref|
JSON|string data|`"hello"`
JSON (accepts)|string data|`"hello"`

### `optional(innerType)`

*New type introduced in 2.1 version.*

Value of optional type `optional(innerType)` can store a value of `innerType` of be empty.
Example: `optional(string)`.

Usage|Value|Examples
---|---|---
Cell|1 bit flag (`1` - value is stored, otherwise `0`)</br>and the value itself (according to `innerType`) if it presents|
JSON|according to `innerType` or `null` if it is empty|`"hello"`
JSON (accepts)|according to `innerType` or `null` if it is empty|`"hello"`

### `itemType[]`

Array of the `itemType` values. Example: `uint256[]`

Usage|Value|Examples
---|---|---
Cell|32 unsigned bit length of the array, 1 bit flag</br>(`0` if array is empty, otherwise `1`) and dictionary of keys and values</br>where key is 32 unsigned bit index and value is `itemType`|
JSON|list of `itemType` values in `[]`|`[1, 2, 3]`, `["hello", "world"]`
JSON (accepts)|list of `itemType` values in `[]`|`[1, 2, 3]`, `["hello", "world"]`
