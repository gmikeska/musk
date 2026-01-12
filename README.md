# Musk

SDK for compiling, deploying, and spending Simplicity contracts on Elements/Liquid networks.

## Overview

Musk provides a high-level Rust interface for working with Simplicity contracts. It wraps the SimplicityHL compiler and provides utilities for:

- Contract compilation and instantiation
- Taproot address generation
- Transaction construction and signing
- Witness value management

## Installation

Add musk to your `Cargo.toml`:

```toml
[dependencies]
musk = { path = "../musk" }
```

## Usage

### Basic Example

```rust
use musk::{Contract, Arguments, WitnessValues};

// Load and compile a contract
let contract = Contract::from_file("my_contract.simf")?;
let compiled = contract.instantiate(Arguments::default())?;

// Generate an address
let address = compiled.address(&elements::AddressParams::ELEMENTS);
println!("Contract address: {}", address);

// Build a spending transaction
let builder = SpendBuilder::new(compiled, utxo)
    .genesis_hash(genesis_hash);

// Compute sighash for witness generation
let sighash = builder.sighash_all()?;

// Create witness values
let witness = WitnessValues::default();

// Finalize and broadcast
let tx = builder.finalize(witness)?;
```

### With Arguments and Witnesses

```rust
use musk::{Contract, Arguments, Value, WitnessName};
use std::collections::HashMap;

// Load contract
let contract = Contract::from_file("p2pk.simf")?;

// Provide arguments
let mut args = HashMap::new();
args.insert(
    WitnessName::from_str_unchecked("ALICE_PUBLIC_KEY"),
    Value::u256(pubkey),
);
let compiled = contract.instantiate(Arguments::from(args))?;

// Create witness with signature
let mut witness = HashMap::new();
witness.insert(
    WitnessName::from_str_unchecked("ALICE_SIGNATURE"),
    Value::byte_array(signature),
);

let tx = builder.finalize(WitnessValues::from(witness))?;
```

## Architecture

Musk is designed to be network-agnostic through the `NodeClient` trait, allowing it to work with regtest, testnet, and mainnet:

```
Contract (source) → TemplateProgram → CompiledContract → Address
                                                        ↓
                                    SpendBuilder → Transaction
```

## Features

- `serde`: Enable serialization support (default)

## License

MIT OR Apache-2.0

