# TON Smart Contracts ABI 2.0 Specification

> **NOTE**: This is an older specification version.
> 
> For the current ABI specification refer to the [ABI.md](ABI.md) file.
>
> All changes between versions are documented in the [Changelog](../CHANGELOG.md).

- [TON Smart Contracts ABI 2.0 Specification](#ton-smart-contracts-abi-20-specification)
	- [New in ABI v2.](#new-in-abi-v2)
- [Motivation](#motivation)
- [Specification](#specification)
	- [Message body](#message-body)
		- [External Inbound Messages](#external-inbound-messages)
		- [External Outbound Messages](#external-outbound-messages)
		- [Internal Messages](#internal-messages)
	- [Message Body Signing](#message-body-signing)
		- [Signing Algorithm](#signing-algorithm)
	- [Function Signature (Function ID)](#function-signature-function-id)
		- [Function Signature Syntax](#function-signature-syntax)
		- [Signature Calculation Syntax](#signature-calculation-syntax)
		- [Sample Implementation](#sample-implementation)
		- [Event ID](#event-id)
	- [Encoding](#encoding)
	- [Header parameter types](#header-parameter-types)
		- [Function parameter types:](#function-parameter-types)
	- [Cell Data Overflow](#cell-data-overflow)
	- [Cell Reference Limit Overflow](#cell-reference-limit-overflow)
	- [Contract Interface Specification](#contract-interface-specification)
		- [Header section](#header-section)
		- [**Functions** section](#functions-section)
		- [Events section](#events-section)
		- [Data section](#data-section)
		- [Getters section](#getters-section)
- [Problem of mappings or arrays that contains "big" structures as values.](#problem-of-mappings-or-arrays-that-contains-big-structures-as-values)
	- [Introduction](#introduction)
	- [Solving of the problem:](#solving-of-the-problem)
  
## New in ABI v2.

ABI v2 introduces a new `header`  JSON ABI section with additional parameters placed before contract function parameters. These additional parameters are used for security checks or some protection mechanisms implemented in a contract. For example, `timestamp` introduced in ABI v1 and used for replay attack protection is now defined as an additional parameter in the `header` section.

Apart from `timestamp`, the new `expire` additional parameter is introduced. It specifies the timespan upon expiration of which a message is not processed by a contract.

Some other minor modifications

- Public key became one of header parameters.
- Signature is moved to root cell.
- Get methods are placed in a separate section that helps find them among other public methods.
- The last cell reference can be used by parameter serialization which needs reference (`cell`, `bytes`, `map`, `array` types) if all the following parameters can fit into current cell. ABI v1 used last cell reference only for cells chaining.

# Motivation

Given the increase in number of additional parameters, it is necessary to review the way they are defined. The `header` section is intended to include all additional parameters that contract expects in external inbound message body for all public functions. These parameters are placed into the cell body before `function ID` in order of appearance in the `header` section.

The public key became an optional parameter in order to decrease message size and therefore to reduce the forward fee. Each contract already has a public key, so there is no need to include it into each message.

Signature is moved to the root cell to decrease forward and gas fees. Given that reading a cell from reference consumes gas, reading the signature directly from the root cell is cheaper. Besides that, an additional cell increases forward fee.

# Specification

ABI specifies message bodies layout for client to contract and contract to contract interaction.

## Message body

### External Inbound Messages

Message body with encoded function call has the following format:

`Maybe(Signature)` +  `Enc(Header)` +`Function ID`+  `Enc(Arguments)`

First comes an optional signature. It is prefixed by one bit flag that indicates the signature presence. If it is `1`, then in the next 512 bit a signature is placed, otherwise the signature is omitted.

Then сomes the encoded header parameters set  (same for all functions).

It is followed by ***32 bits*** of function ID identifying which contract functions are called. The function ID comes within the first 32 bits of the SHA256 hash of the function signature. The highest bit is set to `0` for function ID in external inbound messages, and to `1` for external outbound messages.

Function parameters are next. They are encoded in compliance with the present specification and stored either to the root cell or the next one in the chain.

**Note**: an encoded parameter cannot be split between different cells.

### External Outbound Messages

External outbound messages are used to return values from functions or to emit events.

Return values are encoded and put into the message response:

`Function ID`+`Enc(Return values)`

Function ID's highest bit is set to `1`. 

Events are encoded as follows:

`Event ID + Enc(event args)`

`Event ID` - 32 bits of SHA256 hash of the event function signature with highest bit set to `0`.

### Internal Messages

Internal messages are used for contract-to-contract interaction; they have the following body format:

`Function ID`+`Enc(Arguments)`

`Function ID` - 32 bits function id calculated as first 32 bits SHA256 hash of the function signature. The highest bit of function ID is `0`. Internal messages contain only function calls and no responses.

## Message Body Signing

The message body can be protected with a cryptographic signature to identify a user outside the blockchain. In this case, an *External inbound message* that calls the function carries a user *private key* signature. This requirement applies only to *External inbound messages* because *Internal inbound messages* are generated within the blockchain, and *src address* can be used to identify the caller. 

If a user does not want to sign a message, bit `0` should be placed to the root cell start and signature omitted.

The message body signature is generated from the *representation hash* of the bag of cells following the signature.

### Signing Algorithm

1. ABI serialization generates bag of cells containing header parameters, function ID and function parameters. 513 free bits are reserved in the root cell for signature and signature flag 
2. *Representation hash* of the bag is signed using the *Ed25519* algorithm.
3. Bit `1` followed by 512 bits of the signature is saved to the start of the root cell before header parameters.

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

### Function Signature Syntax

`function_name(input_type1,input_type2,...,input_typeN)(output_type1,output_type2,...,output_typeM)v2`

### Signature Calculation Syntax

`SHA256("function_name(input_type1,input_type2,...,input_typeN)(output_type1,output_type2,...,output_typeM)")v2`

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

## Encoding

The goal of the ABI specification is to design ABI types that are cheap to read to reduce gas consumption and gas costs. Some types are optimized for storing without write access.

## Header parameter types

- `time`: message creation timestamp. Used for replay attack protection, encoded as 64 bit Unix time in milliseconds.

    Rule: the contract should store the timestamp of the last accepted message. The initial timestamp is 0. When a new message is received, the contract should do the following check: 

    `last_time < new_time < now + interval,` where 

    `last_time` - last accepted message timestamp (loaded from c4 register),

    `new_time` - inbound external message timestamp (loaded from message body),

    `now` - current block creation time (just as `NOW` TVM primitive),

    `interval` - 30 min.

    The contract should continue execution if these requirements are met. Otherwise, the inbound message should be rejected.

- `expire`: Unix time (in seconds, 32 bit) after that message should not be processed by contract. It is used for indicating lost external inbound messages.

    Rule:  if contract execution time is less then `expire` time, then execution is continued. Otherwise, the message is expired, and the transaction aborts itself (by `ACCEPT` primitive). The client waits for message processing until the `expire` time. If the message wasn't processed during that interval is considered to be expired

- `pubkey`: public key from key pair used for signing the message body. This parameter is optional. The client decides if he needs to set the public key or not. It is encoded as bit 1 followed by 256 bit of public key if parameter provided, or by bit `0` if it is not.
- Header may also contain any of standard ABI types used by custom checks.

### Function parameter types:

- `uint<M>`: unsigned `M` bit integer. Big-endian encoded unsigned integer stored in the cell-data.
- `int<M>`: two’s complement signed `M` bit integer. Big-endian encoded signed integer stored in the cell-data.
- `bool`: equivalent to uint1.
- tuple `(T1, T2, ..., Tn)`: tuple that includes `T1`, ..., `Tn`, `n>=0` types encoded in the following way:

    ```
    Enc(X(1)) Enc(X(2)) . . ., Enc(X(n)); where X(i) is value of T(i) for i in 1..n 
    ```

    Tuple elements are encoded as independent values so they can be placed in different cells

- `T[]` is a dynamic array of `T` type elements. It is encoded as a TVM dictionary.  `uint32` defines the array elements count placed into the cell body.  `HashmapE` (see TL-B schema in TVM spec) struct is then added (one bit as a dictionary root and one reference with data if the dictionary is not empty). The dictionary key is a serialized `uint32` index of the array element, and the value is a serialized array element as `T` type.
- `T[k]` is a static size array of `T` type elements. Encoding is equivalent to `T[]` without elements count
- `bytes`: an array of `uint8` type elements. The array is put into a separate cell. In the case of array overflow, the maximum cell-data size it's split into multiple sequential cells.
    - Note: contract stores this type as-is without parsing. For high-speed decoding, cut reference from body slice as `LDREF`. This type is helpful if some raw data must be stored in the contract without write or random access to elements.
    - Note: analog of `bytes` in Solidity. In C lang can be used as `void*`.
- `fixedbytes<M>`: a fixed-size array of `M` `uint8` type elements. Encoding is equivalent to `bytes`
- `map(K,V)` is a dictionary of `V` type values with `K` type key. `K` may be any of `int<M>/uint<M>` types with `M` from `1` to `1023`. Dictionary is encoded as  `HashmapE` type (one bit put into cell data as dictionary root and one reference with data is added if the dictionary is not empty).
- `address` is an account address in TON blockchain. Encoded as `MsgAddress` struct (see TL-B schema in TON blockchain spec).
- `cell`: a type for defining a raw tree of cells. Stored as a reference in the current cell. Must be decoded with `LDREF`  command and stored as-is.
    - Note: this type is useful to store payloads as a tree of cells analog to contract code and data in the form of `StateInit` structure of `message` structure.

## Cell Data Overflow

If parameter data does not fit into the available space of the current cell-data, it moves to a separate new cell. This cell is attached to the current one as a reference. The new cell then becomes the current cell.

## Cell Reference Limit Overflow

For simplicity, this ABI version reserves the last cell-reference spot for cell-data overflow. If the cell-reference limit in the current cell is already reached (save for the reserved spot) and a new cell is required, the current cell is considered complete, and a new one is generated. The reserved spot stores the reference to the new cell, and it continues with the new cell as a current one.

The last cell reference can be used by parameter serialization which needs reference (`cell`, `bytes`, `map`, `array` types) if all the following parameters can fit into current cell.

## Contract Interface Specification

The contract interface is stored as a JSON file called contract ABI. It includes all public functions with data described by ABI types. Below is a structure of an ABI file:

```jsx
{
	"ABI version": 2,
	"header": [
		...
	],
	"functions": [
		...		
	],
	"getters": [
	  ...
	],
	"events": [
		...	
	],
	"data": [
		...
	]
}
```

Getters is a list of get methods which might be called on local TVM. 

### Header section

This section describes additional parameters of functions within the contract. Header-specific types are specified as strings with the type `name`. Other types are specified as function parameter type (see **Functions section**) 

```jsx
"header": [
	"header_type",
	{
		"name": "param_name",
		"type": "param_type"
	},
	...
]
```

Example

```jsx
"header": [
	"time",
	"expire",
	{
		"name": "custom",
		"type": "int256"
	}
]
```

### **Functions** section

The **Functions** section specifies each interface function signature, including its name, input, and output parameters. Functions specified in the contract interface can be called from other contracts or from outside the blockchain via ABI call.

**Functions** section has the following fields:

```jsx
"functions": [
	{ 		
		"name": "method_name",
		"inputs": [{"name": "func_name", "type": "ABI_type"}, ..],
		"outputs": [...],
		"id": "0xXXXXXXXX", //optional
	},
	...
]
```

- `name`: function name;
- `inputs`: an array of objects, each containing:
    - `name`: parameter name;
    - `type`: the canonical parameter type.
    - `components`: used for tuple types, optional.
- `id`: an optional `uint32` `id` parameter can be added. This `id` will be used as a `Function ID` instead of automatically calculated. PS: the last case can be used for contracts that are not ABI-compatible.
- `outputs`: an array of objects similar to `inputs`. It can be omitted if the function does not return anything;

### Events section

This section specifies the events used in the contract. An event is an external outbound message with ABI-encoded parameters in the body.

```jsx
"events": [
	{ 		
		"name": "event_name",
		"inputs": [...],
		"id": "0xXXXXXXXX", //optional
	},
	...
]
```

`inputs` have the same format as for functions.

### Data section

This section covers the contract global public variables.

```jsx
"data": [
	{ 		
		"name": "var_name",
		"type": "abi_type",
		"key": "<number>" // index of variable in contract data dictionary
	},
	...
]
```

### Getters section

Getters specification is not yet supported and this section is ignored.

# Problem of mappings or arrays that contains "big" structures as values.

## Introduction

Several months ago we did breaking change in TVM. Opcode DICTSET had worked in this way: if some_data+len(key)+len(value) doesn't fit in one cell (1023 bits) then value are stored in ref of cell. Now if it doesn't fit in one cell opcode will throw exception.

We haven't faced with this problem because solidity compiler doesn't support this feature (mappings or arrays that contain "big" structures as values). We are going to support it but ton-abi throws exception then it generates message.

## Solving of the problem:

To set value in dictionaries (arrays or mappings) we will use opcode DICTSET or DICTSETREF.

if (12 + len(key) + maxPossibleValueLength <= 1023) then we use DICTSET.

else we will use DICTSETREF.

12 = 2 + 10 ≥ 2 + log2(keyLength). See [https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L30](https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L30)

  

Max possible size of value:

- intN/uintN - N bit.
- address - 591 bit. See [https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L107](https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb#L107)

```jsx
anycast_info$_ depth:(#<= 30) { depth >= 1 }
   rewrite_pfx:(bits depth) = Anycast;

addr_var$11 anycast:(Maybe Anycast) addr_len:(## 9) 
   workchain_id:int32 address:(bits addr_len) = MsgAddressInt;

2 +  // 11 
1 + 5 + 30 + // anycast
9 + // addr_len
32 + // workchain_id:int32
512 // address
 = 
591
```

- bool - 1 bit
- bytes/cell - 0 bit
- array - 33 bit
- mapping - 1 bit
- structure = SUM maxPosibleLenght(member) for member in members
