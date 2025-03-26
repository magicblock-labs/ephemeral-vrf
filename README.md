# EphemeralVrf

**EphemeralVrf** is a Verifiable Random Function (VRF) implementation for Solana that provides secure, verifiable randomness for decentralized applications. 
It uses a network of oracles to generate and verify random values on-chain.

## Overview     

EphemeralVrf enables dApps to request unpredictable, tamper-resistant random values that can be verified by anyone.

The implementation follows [RFC 9381](https://datatracker.ietf.org/doc/html/rfc9381), utilizing Curve25519's Ristretto group for elliptic curve operations and Schnorr-like signatures for proof verification. 

## API
- [`Consts`](api/src/consts.rs) – Program constants.
- [`Error`](api/src/error.rs) – Custom program errors.
- [`Instruction`](api/src/instruction.rs) – Declared instructions.
- [`SDK`](api/src/sdk.rs) – Custom program events.
- [`State`](api/src/state) – Program state definitions.
- [`DelegateOracleQueue`](program/src/delegate_oracle_queue.rs) – Delegate an Oracle queue to the delegation program.

## Instructions

- [`RequestRandomness`](program/src/request_randomness.rs) – Request a new random value.
- [`ProvideRandomness`](program/src/provide_randomness.rs) – Provide randomness for a request.
- [`Initialize`](program/src/initialize.rs) – Initialize the program state.
- [`ModifyOracle`](program/src/modify_oracles.rs) – Add or modify oracle information.
- [`InitializeOracleQueue`](program/src/initialize_oracle_queue.rs) – Initialize a new oracle queue.

##  Errors

- Unauthorized – The authority is not authorized to perform the operation.
- RandomnessRequestNotFound – The requested randomness was not found.
- InvalidProof – The provided VRF proof is invalid.

## State

- [`Oracle`](api/src/state/oracle.rs) – Oracle data structure.
- [`Oracles`](api/src/state/oracles.rs) – Collection of oracles.
- [`Queue`](api/src/state/queue.rs) – Oracle queue for randomness requests.

## What is a VRF?

A Verifiable Random Function (VRF) is a cryptographic primitive that maps inputs to verifiable pseudorandom outputs. The key properties of a VRF are:

1. Uniqueness: For a given input and private key, there is exactly one valid output.
2. Verifiability: Anyone with the public key can verify that an output was correctly computed from the input without learning the private key.
3. Pseudorandomness: The output appears random to anyone who doesn't know the private key.

4. In EphemeralVrf, oracles use VRFs to generate random values that can be verified on-chain, ensuring that the randomness is both unpredictable and tamper-resistant.

## VRF Implementation

This repository contains an implementation of a **Verifiable Random Function (VRF)** based on **Curve25519** elliptic curve cryptography, using **HKDF** (HMAC-based Key Derivation Function) for key derivation and **SHA-512** as the hash function. The VRF is designed to allow a party to prove that they know a random value derived from a secret key, with the proof being verifiable by any third party.

### Key Features

- **Curve25519-based VRF**: The VRF is implemented using the **Ristretto group** of Curve25519, offering high security and efficiency.
- **Key Generation**: The secret and public keys are derived using **HKDF**, ensuring secure key generation from an initial keypair.
- **VRF Computation**: The VRF output is computed by hashing the input to a point and applying scalar multiplication. The proof consists of commitments and a response that is verified through a Schnorr-like signature scheme.
- **Proof Verification**: The verification function checks two **Schnorr-like** relations, ensuring the integrity and validity of the VRF proof.

### Cryptographic Primitives

- **Curve25519**: The cryptographic foundation of the VRF, offering a secure elliptic curve with efficient computation and strong security guarantees.
    - **Ristretto group**: Provides non-malleability and robustness in scalar operations.
    - **Scalar multiplication**: Used to generate public keys and VRF outputs.

- **SHA-512**: A strong hash function used throughout the protocol, including in the key derivation and challenge generation.

- **HKDF**: A key derivation function that is based on **HMAC** and used for securely generating secret keys from initial entropy sources.

- **Schnorr-like Signature Scheme**: Used for generating and verifying the VRF proof, ensuring that the output is verifiably bound to the input and secret key.

## Approach

The VRF implementation follows the structure laid out in **RFC 9381**, consisting of the following steps:

1. **Key Generation**: A key pair is derived from a given keypair using **HKDF** to generate a secret key (`sk`) and a corresponding public key (`pk`), which is a scalar multiple of the base point on Curve25519.

2. **VRF Computation**:
    - The input is hashed to a point using the `hash_to_point` function.
    - The output of the VRF is computed by multiplying the secret key (`sk`) with the hashed point.
    - A nonce (`k`) is derived, and commitments are computed for both the base point and the hashed point.
    - A challenge value is generated by combining various elements (output, commitments, etc.) and hashing them. The final response (`s`) is computed using the standard Schnorr signature response formula.

3. **VRF Proof Verification**:
    - The verifier recomputes the challenge and checks two **Schnorr-like** relations:
        - **Base point check**: `s * G == commitment_base + c * pk`
        - **Hashed point check**: `s * h == commitment_hash + c * output`
    - If both checks pass, the proof is valid.

## Soundness

The security of the VRF relies on the hardness of the **Discrete Logarithm Problem (DLP)** in elliptic curve cryptography. The implementation ensures that:

1. **Correctness**: The VRF proof is guaranteed to be correct if the two Schnorr-style checks hold.
2. **Unforgeability**: An adversary cannot generate a valid proof without knowledge of the secret key.
3. **Binding**: The output is bound to the input, ensuring that the same input always produces the same output and proof.
4. **Non-malleability**: The proof cannot be altered or manipulated without invalidating the verification.

## Warning

The code is not audited and should not be used in production environments without thorough testing and security analysis.

## Get started

Compile your program:
```sh
cargo build-sbf
```

Run unit and integration tests:
```sh
cargo test-sbf --features test-sbf
```

Run the oracle service:
```sh
RUST_LOG=info cargo run --bin vrf-oracle 
```

## Oracle CLI

CLI for managing oracles. See all available commands with:

```bash
cargo run --bin vrf-cli -- --help
```

## Example Usage

See the [integration tests](program/tests/integration/use-randomness/programs/use-randomness/src/lib.rs) for example usage of the program.

