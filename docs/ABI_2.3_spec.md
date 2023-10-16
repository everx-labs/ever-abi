# Everscale Smart Contracts ABI 2.3 Specification

> **NOTE**: This is an older specification version.
> 
> For the current ABI specification refer to the [ABI.md](ABI.md) file.
>
> All changes between versions are documented in the [Changelog](../CHANGELOG.md).

ABI 2.3 introduces the new method to calculate external inbound message body signature. It is aimed to fix the below described problem in ABI v2.2.   
Big thanks to Everscale community member https://github.com/mnill Ilia Kirichek who found these problems.

## Problem
External messages may have a signature. Signatures are dependent only on message body (without signature flag and signature itself). Signatures aren’t dependent on `dest` address and it may cause a problem. Let's consider the following situation:  

1. User has 2 contracts that contain the same public key and public function with same signature
2. User sends message to the first contract
3. Then hacker can create a message and send it to the second contract and it may be successful.

To solve the problem signature must be dependent on the destination address.

## Modified Signing Algorithm

1. ABI serialization generates bag of cells containing header parameters, function ID and function parameters.
591 free bits are reserved in the root cell for destination address (the maximum size of internal address).
2. The root cell data is prepended with actual destination address data without padding to maximum size.
3. *Representation hash* of the bag is signed using the *Ed25519* algorithm.
4. Address data is removed from the root cell and replaced with bit `1` followed by 512 bits of the signature.

> This fucntionality is supported staring with [0.64.0](https://github.com/tonlabs/EVERX-Solidity-Compiler/blob/master/Changelog_EVERX.md#0640-2022-08-18) version of the Solidity compiler.


