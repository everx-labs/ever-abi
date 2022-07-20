# Everscale Smart Contracts ABI 2.3 Specification

ABI 2.3 introduces the new method to calculate external inbound message body signature. It is aimed to fix the following problem in ABI v2.2.   
Big thanks to Everscale community member https://github.com/mnill Ilia Kirichek who found these problems.

## Problem
External messages may have a signature. Signatures respect only message body (without signature flag and signature itself). Signatures don’t respect `dest` address and it may cause a problem. Let's consider situation:  

1. User have 2 contracts that contain same public key and public function with same signature
2. User sends message to the first contract
3. Then hacker can create message and sends it to the second contract and it may be successful.

To solve the problem signature must respect destination address.

## Modified Signing Algorithm

1. ABI serialization generates bag of cells containing header parameters, function ID and function parameters.
591 free bits are reserved in the root cell for destination address (the maximum size of internal address)
2. The root cell data is prepended with actual destination address data without padding to maximum size.
3. *Representation hash* of the bag is signed using the *Ed25519* algorithm.
4. Address data is removed from the root cell and replaced with bit `1` followed by 512 bits of the signature.


