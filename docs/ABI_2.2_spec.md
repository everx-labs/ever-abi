# EVERX Smart Contracts ABI 2.2 Specification

> **NOTE**: This is an older specification version.
> 
> For the current ABI specification refer to the [ABI.md](ABI.md) file.
>
> All changes between versions are documented in the [Changelog](../CHANGELOG.md).

ABI 2.2 introduces the new fixed message body layout while sections and types stay the same as in [ABI 2.1](./ABI_2.1_spec.md). Read below. 
 
- [EVERX Smart Contracts ABI 2.2 Specification](#ever-smart-contracts-abi-22-specification)
	- [Fixed layout concepts](#fixed-layout-concepts)
	- [Introduction](#introduction)
	- [Encoding the message](#encoding-the-message)
	- [Encoding the body of the message](#encoding-the-body-of-the-message)
		- [Encoding header for external messages](#encoding-header-for-external-messages)
		- [Encoding of function ID and its arguments](#encoding-of-function-id-and-its-arguments)


## Fixed layout concepts

Since ABI v2.2 fixed message body layout is used in order to reduce gas consumption while parsing parameters. This document describes fixed layout concepts.

## Introduction

Each type has max bit and max ref size:

- `intN/uintN` - N bits, 0 refs
- `varint16/varuint16` - 124 bits, 0 refs
- `varint32/varuint32` - 253 bits, 0 refs
- `address` - 591 bits, 0 refs
- `bool` - 1 bit, 0 refs
- `bytes/cell/string` - 0 bit, 1 ref
- `array` - 33 bit, 1 ref
- `mapping` - 1 bit, 1 ref
- `optional(T)` - (1 bit, 1 ref) if `optional` is [large](ABI_2.1_spec.md#optionalinnertype). Otherwise, (1 bit + maxBitQty(`T`), maxRefQty(`T`))

`structure (aka tuple)` type is considered as a sequence of its types when we encode the function parameters. That's why `tuple` type doesn't have max bit or max ref size. Nested `tuple` 's also are considered as a sequence of its types. For example:

```jsx
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

## Encoding the message

`Message X` contains the field `body`. If encoded `body` fits in the cell, then the body is inserted in the cell (`Either X`). Otherwise, `body` is located in the reference (`Either ^X`). 

## Encoding the body of the message

The body of the message is a tree of cells that contains the function ID and encoded function arguments. External messages body is prefixed with function header parameters.

### Encoding header for external messages

Function header has up to 3 optional parameters and mandatory signature. Function ID and function parameters are put after header parameters.

Maximum header size is calculated as follows (no references used).

```jsx
maxHeader = 1 + 512 + // signature
    (hasPubkey? 1 + 256 : 0) +
    (hasTime? 64 : 0) +
    (hasExpire? 32 : 0);
```


### Encoding of function ID and its arguments

Function ID and the function arguments are located in the chain of cells. The last reference of each cell (except for the last cell in the chain) refers to the next cell. After adding the current parameter in the current cell we must presume an invariant (rule that stays true for the object) for our cell: number of unassigned references in the cell must be not less than 1 because the last cell is used for storing the reference on the next cell. When we add a specific value of some function argument to the cell we assume that it takes the max bit and max ref size. Only if the current parameter (by max bit or max ref size) does not fit into the current cell then we create new cell and insert the parameter in the new cell. 

***But*** If current argument and all the following arguments fit into the current cell by max size then we push the parameters in the cell.

In the end we connect the created cells in the chain of cells.

For example:

```jsx
function f(address a, address b) public;
```

Here we create 2 cells. In the first there is function id and  `a`. There may be not more than 32+591=623 bits. It's not more than 1023. The next parameter `b` can't fit into the first cell. In the second cell there is only `b`.

```jsx
function f(mapping(uint=>uint) a, mapping(uint=>uint) b, mapping(uint=>uint) c, mapping(uint=>uint) d)
```

The first cell: function ID, `a`, `b` `c`, `d`. 

```jsx
function f(string a, string b, string c, string d, uint32 e) public
```

Function ID, `a`, `b`, `c` are located in the first cell. `d` and `e` fit in the first cell by max size. That's why we push all parameter in the fist cell.

```jsx
struct A {
	string a;
	string b;
	string c;
	string d;
}

function f(A a, uint32 e) public;
```

Same as previous example, only one cell.

```jsx
function f(string a, string b, string c, string d, uint e, uint f, uint g, uint h) public
```

We use 3 cells. In the first cell there are function Id, `a`, `b,` `c`. In the second - `d`, `e`, `f`, `g`. In the third - `h`.

