# Smart Contracts ABI v2.3 Specification

ABI specifies message bodies layout for client to contract and contract to contract interaction.

## Introduction

In Everscale client to contract and contract to contract interaction occurs through external and internal messages respectively.

ABI specification describes the structure of body of these messages. ABI stored as JSON serves as an interface for smart contracts and is used when calling contract methods externally or on-chain.

The goal of the ABI specification is to design ABI types that are cheap to read to reduce gas consumption and gas costs. Some types are optimized for storing without write access.

## Message body

### External Inbound Messages

Message body with encoded function call has the following format:

`Maybe(Signature)` +  `Enc(Header)` +`Function ID` +  `Enc(Arguments)`

First comes an optional signature. It is prefixed by one bit flag that indicates the signature presence. If it is `1`, then in the next `512 bit` a signature is placed, otherwise the signature is omitted.

Then comes the encoded header parameters set (same for all functions).

It is followed by ***32 bits*** of function ID identifying which contract functions are called. The `function ID` comes within the first `32 bits` of the `SHA256` hash of the function signature.

The highest bit is set to `0` for function ID in external inbound messages, and to `1` for external outbound messages.

Function parameters are next. They are encoded in compliance with the present specification and stored either in the root cell or the next one in the chain.

:::note
An encoded parameter cannot be split between different cells
:::

### External Outbound Messages

External outbound messages are used to return values from functions or to emit events.

Return values are encoded and put into the message response:

`Function ID`+`Enc(Return values)`

Function ID's highest bit is set to `1`.

Events are encoded as follows:

`Event ID` + `Enc(event args)`

`Event ID` - 32 bits of SHA256 hash of the event function signature with highest bit set to `0`.

### Internal Messages

Internal messages are used for contract-to-contract interaction; they have the following body format:

`Function ID` + `Enc(Arguments)`

`Function ID` - 32 bits function id calculated as first 32 bits SHA256 hash of the function signature. The highest bit of function ID is `0`. Internal messages contain only function calls and no responses.

## Message Body Signing

The message body can be protected with a cryptographic signature to identify a user outside the blockchain. In this case, an *External inbound message* that calls the function carries a user *private key* signature. This requirement applies only to *External inbound messages* because *Internal inbound messages* are generated within the blockchain, and *src address* can be used to identify the caller.

If a user does not want to sign a message, bit `0` should be placed to the root cell start and signature omitted.

The message body signature is generated from the *representation hash* of the bag of cells following the signature prepended with src address.

## Signing Algorithm

1. ABI serialization generates bag of cells containing header parameters, function ID and function parameters.
   591 free bits are reserved in the root cell for destination address ([the maximum size of address](#address)).
2. The root cell data is prepended with actual destination address data without padding to maximum size.
3. *Representation hash* of the bag is signed using the *Ed25519* algorithm.
4. Address data is removed from the root cell and replaced with bit `1` followed by 512 bits of the signature.

:::note
This functionality is added since `ABI v2.3` and supported staring with [0.64.0](https://github.com/tonlabs/TON-Solidity-Compiler/blob/master/Changelog_TON.md#0640-2022-08-18) version of the Solidity compiler.
:::

## Function Signature (Function ID)

The following syntax is used for defining a signature:

- function name
- list of input parameter types (input list) in parenthesis
- list of return values types (output list) in parenthesis
- ABI version

Single comma is used to divide each input parameter and return value type from one another. Spaces are not used.

Parameter and return value names are not included.

The function name, input and output lists are not separated and immediately follow each other.

If a function has no input parameters or does not return any values, the corresponding input or output lists are empty (empty parenthesis).

 Function ID may be indicated in ABI separately. Then the first bit stays the same regardless of incoming/outgoing message.

### Function Signature Syntax

`function_name(input_type1,input_type2,...,input_typeN)(output_type1,output_type2,...,output_typeM)v2`

### Signature Calculation Syntax

`SHA256("function_name(input_type1,input_type2,...,input_typeN)(output_type1,output_type2,...,output_typeM)v2")`

### Sample Implementation

**Function**

`func(int64 param1, bool param2) -> uint32`

**Function Signature**

`func(int64,bool)(uint32)v2`

**Function Hash**

`sha256("func(int64,bool)(uint32)v2") = 0x1354f2c85b50aa84c2f65ebb8cec69aba0aa3269c21e03e142e014e84ea59649`

**function ID** then is `0x1354f2c8` for function call and `0x9354f2c8` for function response

### Event ID

**Event ID** is calculated in the same way as the **function ID** except for cases when the event signature does not contain the list of return values types: `event(int64,bool)v2`

## Header parameter types

- [`time`](#time): message creation timestamp. Encoded as 64 bit Unix time in milliseconds.

- [`expire`](#expire): Unix time (in seconds, 32 bit) after that message should not be processed by contract.

- [`pubkey`](#pubkey): public key from key pair used for signing the message body. This parameter is optional.

**Note**: Header may also contain any of standard function parameter types described below to be used in custom checks.

## Function parameter types

- [`int<N>`](#intn): two’s complement signed `N` bit integer. Big-endian encoded signed integer stored in the cell-data.
- [`uint<N>`](#uintn): unsigned `N` bit integer. Big-endian encoded unsigned integer stored in the cell-data.
- [`varint<N>`](#varintn): variable-length signed integer. Bit length is between `log2(N)` and `8 * (N-1)`, where `N` is equal to 16 or 32.
- [`varuint<N>`](#varuintn): variable-length unsigned integer with bit length equal to 8 * N, where Nis equal to 16 or 32 e.g. Processed like `varint<N>`.
- [`bool`](#bool): equivalent to uint1.
- [tuple `(T1, T2, ..., Tn)`](#tuple): tuple that includes `T1`, ..., `Tn`, `n>=0` types encoded in the following way:

  ```
  Enc(X(1)) Enc(X(2)) ..., Enc(X(n)); where X(i) is value of T(i) for i in 1..n 
  ```

  Tuple elements are encoded as independent values so they can be placed in different cells

- [`map(K,V)`](#mapkeytypevaluetype) is a dictionary of `V` type values with `K` type key. Dictionary is encoded as  `HashmapE` type (one bit put into cell data as dictionary root and one reference with data is added if the dictionary is not empty).
- [`cell`](#cell): a type for defining a raw tree of cells. Stored as a reference in the current cell. Must be decoded with `LDREF`  command and stored as-is.
    - Note: this type is useful to store payloads as a tree of cells analog to contract code and data in the form of `StateInit` structure of `message` structure.
- [`address`](#address) is an account address in Everscale blockchain. Encoded as `MsgAddress` struct (see TL-B schema in blockchain [spec](https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L107)).
- [`bytes`](#bytes): an array of `uint8` type elements. The array is put into a separate cell. 
- [`fixedbytes<N>`](#fixedbytesn): a fixed-size array of `N` `uint8` type elements. Encoding is equivalent to `bytes`
- [`string`](#string) - a type containing UTF-8 string data, encoded like `bytes`.
- [`optional`](#optionalinnertype) - value of optional type `optional(innerType)` can store a value of `innerType` or be empty.
- [`itemType[]`](#itemtype) is a dynamic array of `itemType` type elements. It is encoded as a TVM dictionary.  `uint32` defines the array elements count placed into the cell body.  `HashmapE` (see TL-B schema in TVM spec) struct is then added (one bit as a dictionary root and one reference with data if the dictionary is not empty). The dictionary key is a serialized `uint32` index of the array element, and the value is a serialized array element as `itemType` type.
  - `T[k]` is a static size array of `T` type elements. Encoding is equivalent to `T[]` without elements count.

## Encoding of function ID and its arguments

Function ID and the function arguments are located in the chain of cells. The last reference of each cell (except for the last cell in the chain) refers to the next cell. After adding the current parameter in the current cell we must presume an invariant (rule that stays true for the object) for our cell: number of unassigned references in the cell must be not less than 1 because the last reference is used for storing the reference on the next cell. The last cell in the chain can use all 4 references to store argument's values.

When we add a specific value of some function argument to the cell we assume that it takes the max bit and max ref size for a particular argument type (see [`types reference`](#types-reference) section). Only if the current parameter (by max bit or max ref size) does not fit into the current cell do we create a new cell and insert the parameter in the new cell. But if the current argument and all the following arguments fit into the current cell by max size, then we push the parameters in the cell. The serialized argument value takes up only the necessary bits and refs size without aligning to max sizes of its type.

In the end we connect the created cells in the chain of cells by assigning the last reference in each cell to next cell.

Below are some examples:

```solidity
function f(address a, address b) public;
```

Here we create 2 cells. In the first cell there is function id and  `a`. There may be not more than 32+591=623 bits (591 bits is the [maximum size of 'address'](#address)). So it is not more than 1023 bits. The next parameter `b` thus can't fit into the first cell. In the second cell there is only `b`.

```solidity
function f(mapping(uint=>uint) a, mapping(uint=>uint) b, mapping(uint=>uint) c, mapping(uint=>uint) d)
```

[map](#mapkeytypevaluetype) type takes up maximum 1 bit and 1 ref so all parameters can fit into one cell: function ID, `a`, `b` `c`, `d`.

```solidity
struct A {
  string a;
  string b;
  string c;
  string d;
}

function f(A a, uint32 e) public;
```

Same as the previous example, this fits in one cell because [string](#string) takes 32 bits and 1 ref.

```solidity
function f(string a, string b, string c, string d, uint32 e) public
```

Function ID, `a`, `b`, `c` are located in the first cell. `d` and `e` fit in the first cell by max size. That's why we push all parameters in the fist cell.


```solidity
function f(string a, string b, string c, string d, uint e, uint f, uint g, uint h) public
```

`uint` in Solidity is equal to `uint256`. We use 3 cells. In the first cell there are function Id, `a`, `b,` `c`. In the second - `d`, `e`, `f`, `g`. In the third - `h`.


## Encoding header for external messages

External message's body contains function call header in addition to function ID and arguments. Header has up to 3 optional parameters and mandatory signature. Function ID and function parameters are put after header parameters.

Maximum header size is calculated as follows (no references used).

```js
maxHeader =
  591 +
  (hasPubkey ? 1 + 256 : 0) +
  (hasTime ? 64 : 0)  +
  (hasExpire ? 32 : 0);
```
591 bits are reserved for message destination address to use it while [signing](#signing-algorithm) the body.

Let's look at some examples of header encoding. Assume that header contains `time` and `expire` parameters. It requires `591 + 64 + 32 = 687` bits

```solidity
function f(address a, address b) public;
```

Now we have to use 3 cells. In the first cell we put header and function ID. Parameter `a` can not fit in first cell so it goes to second and `b` is put in the third cell.

```solidity
function f(mapping(uint=>uint) a, mapping(uint=>uint) b, mapping(uint=>uint) c, mapping(uint=>uint) d)
```

Here header and all arguments fit in the first cell. After signing it will contain 645 bits and 4 refs.


## ABI JSON

The contract interface is stored as a JSON file called contract ABI. It includes all public functions with data described by ABI types. Below is a structure of an ABI file in TypeScript notation:

```typescript
type Abi = {
  version: string,
  header?: HeaderParam[],
  functions: Function[],
  events?: Event[],
  data?: Data[],
  fields?: Param[],
}

type HeaderParam = Param | string

type Function = {
  name: string,
  inputs?: Param[],
  outputs?: Param[],
  id?: number,
}

type Event = {
  name: string,
  inputs?: Param[],
  id?: number,
}

type Data = Param & {
  key: number,
}

type Param = {
  name: string,
  type: string,
  components?: Param[],
}
```

### Header

This section describes additional parameters of functions within the contract. Header-specific types are specified as strings with the type `name`. Other types are specified as function parameter type (see [Functions](#function-parameter-types)))

```json
{
  "header": [
    "header_type",
    {
      "name": "param_name",
      "type": "param_type"
    }
  ]
}
```

Example

```json5
{
  "header": [
    "time",
    "expire",
    {
      "name": "custom",
      "type": "int256"
    }
  ]
}
```

### Functions

Specifies each interface function signature, including its name, input, and output parameters. Functions specified in the contract interface can be called from other contracts or from outside the blockchain via ABI call.

Functions section has the following fields:

```json5
{
  "functions": [
    {
      "name": "method_name",
      "inputs": [
        {"name": "func_name", "type": "ABI_type"},
      ],
      "outputs": [],
      "id": "0xXXXXXXXX", //optional
    }
  ]
}
```

- `name`: function name;
- `inputs`: an array of objects, each containing:
    - `name`: parameter name;
    - `type`: the canonical parameter type.
    - `components`: used for tuple types, optional.
- `id`: an optional `uint32` `id` parameter can be added. This `id` will be used as a `Function ID` instead of automatically calculated. PS: the last case can be used for contracts that are not ABI-compatible.
- `outputs`: an array of objects similar to `inputs`. It can be omitted if the function does not return anything;

### Events

This section specifies the events used in the contract. An event is an external outbound message with ABI-encoded parameters in the body.

```json5
{
  "events": [
    {
      "name": "event_name",
      "inputs": [],
      "id": "0xXXXXXXXX", //optional
    },
  ]
}
```

`inputs` have the same format as for functions.

### Data

This section covers the contract global public variables. Data is typically used when deploying multiple identical contracts with the same deployer keys. It affects the contract address, and thus varying data results in unique addresses for identical contracts.

```json5
{
  "data": [
    {
      "name": "var_name",
      "type": "abi_type",
      "key": "<number>" // index of variable in contract data dictionary
    },
  ]
}
```

### Fields

This section describes internal structure of the smart contracts data.

Data structure is described as a list of variables' names with corresponding data types.
It includes contract state variables and some internal contract specific hidden variables.
They are listed in the order in which they are stored in the data field of the contract.
Example for a Solidity contract [BankClient](https://github.com/tonlabs/samples/blob/master/solidity/5_BankClient.sol):

Contract state variables:

```solidity
contract BankClient {
  uint public creditLimit = 0;  // allowed credit limit;
  uint public totalDebt = 0;    // contract total debt;
  uint public balance = 0;    // contract balance;
  uint public value = 0;      // inbound message value.
}
```

Fields section of the abi file:

```json
{
  "fields": [
    {"name":"_pubkey","type":"uint256"},
    {"name":"_timestamp","type":"uint64"},
    {"name":"_constructorFlag","type":"bool"},
    {"name":"creditLimit","type":"uint256"},
    {"name":"totalDebt","type":"uint256"},
    {"name":"balance","type":"uint256"},
    {"name":"value","type":"uint256"}
  ]
}
```

### Types Reference

#### `time`

*Header parameter type.*

`time` is the message creation timestamp. Used for **replay attack protection**, encoded as 64 bit Unix time in milliseconds.

| Usage          | Value    | Examples   | Max bit size | Max ref size |
|----------------|----------|------------|--------------|--------------|
| Cell           | 64 bit, big endian     |  | 64 bits | 0 refs|
| JSON object    |  string with hex or decimal representation    |  `"1685634471"` | | |
 **Rule**: the contract should store the timestamp of the last accepted message. The initial timestamp is 0. When a new message is received, the contract should do the following check:

  `last_time` < `new_time` < `now + interval`, where

  `last_time` - last accepted message timestamp (loaded from c4 register),

  `new_time` - inbound external message timestamp (loaded from message body),

  `now` - current block creation time (just as NOW TVM primitive),

  `interval` - 30 min.

  The contract should continue execution if these requirements are met. Otherwise, the inbound message should be rejected.


#### `expire`

*Header parameter type.*

Unix time (in seconds, 32 bit) after which message should not be processed by contract. It is used for indicating lost external inbound messages.

| Usage          | Value    | Examples   | Max bit size | Max ref size |
|----------------|----------|------------|--------------|--------------|
| Cell           | 32 bit, big endian     |  | 32 bits | 0 refs|
| JSON object    |  string with hex or decimal representation    | `"3600"` | | |

  **Rule**:  if contract execution time is less then `expire` time, then execution is continued. Otherwise, the message is expired, and the transaction aborts itself (by `ACCEPT` primitive). The client waits for message processing until the `expire` time. If the message wasn't processed during that interval it is considered to be expired.


#### `pubkey`

*Header parameter type.*

Public key from key pair used for signing the message body. This parameter is optional. The client decides if they need to set the public key or not. It is encoded as bit 1 followed by 256 bit of public key if parameter provided, or by bit `0` if it is not.

| Usage          | Value    | Examples   | Max bit size | Max ref size |
|----------------|----------|------------|--------------|--------------|
| Cell           | 1 bit, `0` or `1` + 256 bit key if if first bit=1     |  | 257 bit| 0 refs |
| JSON object    | string hexadecimal representation of byte array       | `"33a2ed7a92bb55b3aabe1185d0107d48 faa798246c95ed76f262d857c3d1227b"` | | |

#### `int<N>`

Fixed-sized signed integer, where `N` is a decimal bit length. Examples: `int8`, `int32`, `int256`.

| Usage          | Value    | Examples              | Max bit size | Max ref size |
|----------------|--------------------|-----------------|---|---|
| Cell           | N bit, big endian       | | N bits    |  0 refs   |
| JSON (returns)          | string with hex or decimal representation                              | `"0x12"`, `"100"`                | | |
| JSON (accepts) | number or string with hex or decimal representation | `12`, `"0x10"`, `"100"` | | |

#### `uint<N>`

Fixed-sized unsigned integer, where N is a decimal bit length e.g., `uint8`, `uint32`, `uint256`.
Processed like `int<N>`.

#### `varint<N>`

Variable-length signed integer. Bit length is between `log2(N)` and `8 * (N-1)`, where `N` is equal to 16 or 32, e.g. `varint16`, `varint32`.

| Usage          | Value        | Examples              | Max bit size | Max ref size |
|----------------|---|---|------------------------------------------------------------|-----------------------|
| Cell           | 4 (N=16) of 5 (N=32) bits that encode byte length of the number `len`<br/>followed by `len * 8` bit number in big endian |  | `varint16` type — 124 bits, `varint32` type — 253 bits, etc. | 0 refs |
| JSON (returns)          | string with hex or decimal representation                                                                                    | `"0x12"`, `"100"`                | | |
| JSON (accepts) | number or string with hex or decimal representation                                                              | `12`, `"0x10"`, `"100"` | | |

#### `varuint<N>`

Variable-length unsigned integer with bit length equal to `8 * N`, where `N`is equal to 16 or 32 e.g., `varint16`, `varint32`.
Processed like `varint<N>`.

#### `bool`

Boolean type.

| Usage          | Usage                                          | Examples               | Max bit size | Max ref size |
|----------------|------------------------------------------------|------------------------|---|---|
| Cell           | 1 bit, `0` or `1`                              |                        |1 bit | 0 refs |
| JSON (returns)          | `true`, `false`                                |                        | | |
| JSON (accepts) | `true`, `false`, `0`, `1`, `"true"`, `"false"` | `0`, `true`, `"false"` | | |

#### `tuple`

Struct type, consists of fields of different types. All fields should be specified as an array in the `components` section of the type.

`structure (aka tuple)` type is considered as a sequence of its types when we encode the function parameters. That's why `tuple` type doesn't have max bit or max ref size. Nested `tuple`'s also are considered as a sequence of its types. For example:

```solidity
struct A {
  uint8 a;
  uint16 b;
}

struct B {
  uint24 d;
  A a;
  uint32 d;
}
```

structure `B` is considered as a sequence of `uint24`, `uint8`, `uint16`, `uint32` types.

For example, for structure `S`:
```solidity
struct S {
  uint32 a;
  uint128 b;
  uint64 c;
}
```

parameter `s` of type `S` would be described like:
```json
{
  "components": [
    {"name":"a","type":"uint32"},
    {"name":"b","type":"uint128"},
    {"name":"c","type":"uint64"}
  ],
  "name":"s",
  "type":"tuple"
}
```

| Usage          | Value                                                                                                 | Examples                     |
|----------------|-------------------------------------------------------------------------------------------------------|------------------------------|
| Cell           | chain of cells with tuple data types encoded consistently<br/>(without splitting value between cells) |                              |
| JSON object      | dictionary of struct field names with their values                                                    | `{"a": 1, "b": 2, "c": 3}`   |


#### `map(<keyType>,<valueType>)`

Hashtable mapping keys of `keyType` to values of the `valueType`, e.g., `map(int32, address)`. Key may be any of `int<N>/uint<N>` types with `N` from `1` to `1023` or address of std format.

| Usage          | Value                                                                              | Examples               | Max bit size | Max ref size |
|----------------|------------------------------------------------------------------------------------|-------------------------------------------|---|---|
| Cell           | 1 bit (`0` - for empty mapping, otherwise `1`) and ref to the cell with dictionary |                                           | 1 bit | 1 ref | 
| JSON object | dictionary of keys and values          | `{"0x1":"0x2"}`, `{"2":"3","3":"55"}`     | | |

There are some specifics when working with "big" structures as values in mappings. Read [below](#big-structures-as-values-in-mappings-and-arrays) how to implement them correctly.

#### `cell`

TVM Cell type.

| Usage          | Value                     | Examples                 | Max bit size | Max ref size |
|----------------|---------------------------|----------|---|---|
| Cell           | stored in a ref|          | 0 bit | 1 ref | 
| JSON object           | cell serialized into boc and encoded in base64| `"te6ccgEBAQEAEgAAH/////////////////////g="` | | |

#### `address`

Contract address in type `address`, can be any of the [existing variants](../arch/40-accounts.md#account-address) (although not all may be supported by the compilator you are using).

**Important notes:**

1. All hexadecimal values represented in **lower case**.
2. Bitstrings are represented in hexadecimal variable length form with `_` suffix if length is not multiple of 4. When length is multiple of 4 bitstring is always encoded **without** `_` suffix.

**Format**

```jsx
"" // None
":A...A" // External
"[N..N:]W:A...A" // Internal
```

where:

- `W` is a decimal signed representation for workchain_id.
- `A...A` is a string representation of bitstring (see important nodes above);
- `N...N` is a string representation of bitstring with anycast rewrite prefix.

**Serialization**

Internal addresses are serialised as:

- `std` when workchain id is 8-bit and address is 256-bit
- `var` otherwise.

**Size**

Maximum size allocated for address is 591 bits: see [https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L107](https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L107)

```TL-B
anycast_info$_ depth:(#<= 30) { depth >= 1 }
   rewrite_pfx:(bits depth) = Anycast;

addr_var$11 anycast:(Maybe Anycast) addr_len:(## 9) 
   workchain_id:int32 address:(bits addr_len) = MsgAddressInt;

2 +          // 11 
1 + 5 + 30 + // anycast
9 +          // addr_len
32 +         // workchain_id:int32
512          // address
 = 
591
```

| Usage       | Value      | Examples    | Max bit size | Max ref size |
|----------------|---------------------|---------|---|---|
| Cell           | 2 bits of address type, 1 bit of anycast, wid - 8 bit signed integer and address value - 256 bit unsigned integer |                                                                          | 591 bits | 0 refs |
| JSON object         | string                       | `"123:000000000000000000000000000000 000000000000000000000000000001e0f3"` | | |


#### `bytes`

An array of `uint8` type elements. The array is put into a separate cell. In the case of array overflow, the maximum cell-data size it's split into multiple sequential cells.

**Note**: contract stores this type as-is without parsing. For high-speed decoding, cut reference from body slice as `LDREF`. This type is helpful if some raw data must be stored in the contract without write or random access to elements.

Analog of `bytes` in Solidity. In C lang can be used as `void*`.

| Usage          | Value                          | Examples   | Max bit size | Max ref size |
|----------------|--------------------------------|------------|---|---|
| Cell           | cell with data stored in a ref |            | 0 bit | 1 ref |
| JSON object    | binary daya represented as hex string | `"313233"` | | |

#### `fixedbytes<N>`

Where N is a decimal byte length from 1 to 32. It is denoted in abi as `uint<M>`,
where `M` is a bit length and `M = 8 * N`.
Processed like `int<N>`.

#### `string`

UTF-8 String data. Encoded like `bytes`. In JSON is represented as a sting.

| Usage           | Value                          | Examples  | Max bit size | Max ref size |
|-----------------|--------------------------------|-----------|--|--|
| Cell            | cell with data stored in a ref |           | 0 bit | 1 ref |
| JSON object     | string data                    | `"hello"` | | |

#### `optional(innerType)`

Value of optional type `optional(innerType)` can store a value of `innerType` or be empty.

Example: `optional(string)`.

The optional type is a  `large` if `maxBitSize(InnerType) + 1 > 1023 || maxRefSize(InnerType) >= 4`.

Large optional values are always stored as a reference. The optional bit itself is stored on the main branch.

Small optional values are stored in the same cell with the optional bit.

| Usage          | Value      | Examples    | Max bit size | Max ref size |
|----------------|---------------|-------------------|---|---|
| Cell           | 1 bit flag (`1` - value is stored, otherwise `0`) and the value itself (according to `innerType`) if it presents | | 1 bit if `optional` is large, `1 bit + maxBitQty(T), maxRefQty(T)` otherwise | 1 ref if `optional` is large, 0 refs otherwise | 
| JSON object    | according to `innerType` or `null` if it is empty                                                                    | `"hello"`   | | |

#### `itemType[]`

Array of the `itemType` values. Example: `uint256[]`

| Usage          | Value       | Examples      | Max bit size | Max ref size |
|----------------|-------------|---------------|---|---|
| Cell           | 32 unsigned bit length of the array, 1 bit flag (`0` if array is empty, otherwise `1`) and dictionary of keys and values where key is 32 unsigned bit index and value is `itemType` |                                   | 33 bit | 1 ref|
| JSON object      | list of `itemType` values in `[]`  | `[1, 2, 3]`, `["hello", "world"]` | | |

There are some specifics when working with "big" structures as values in arrays. Read [below](#big-structures-as-values-in-mappings-and-arrays) how to implement them correctly.

### "Big" structures as values in mappings and arrays

When working with "big" structures in mappings and arrays data may be written in two possible ways - either into cell or into reference, depending on the size:

```
if (12 + len(key) + maxValueBitLength <= 1023) then write data into cell

else write data to reference.

12 = 2 + 10 ≥ 2 + log2(keyLength).
```

See [https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L30](https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L30)

## Reference

- [ABI changelog specifications](https://github.com/tonlabs/ever-abi/tree/master/docs)
- [ABI implementation](https://github.com/tonlabs/ever-abi)
- [ABI parser](https://github.com/broxus/everscale-web-tools/tree/master/abi-parser)
- [ABI serializer](https://ever.bytie.moe/serializer)
