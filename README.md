# Musk

SDK for compiling, deploying, and spending Simplicity contracts on Elements/Liquid networks.

## Overview

Musk provides a high-level Rust interface for working with Simplicity contracts. It wraps the SimplicityHL compiler and provides utilities for:

- Contract compilation and instantiation
- Taproot address generation
- Transaction construction and signing
- Witness value management
- **Node connectivity via RPC**

## Installation

Add musk to your `Cargo.toml`:

```toml
[dependencies]
musk = { path = "../musk" }
```

## Usage

### Connecting to Nodes

Musk provides an `RpcClient` for connecting to Elements/Liquid nodes:

```rust
use musk::{NodeConfig, RpcClient};

// Method 1: Load from config file
let client = RpcClient::from_config_file("musk.toml")?;

// Method 2: Create programmatically
let config = NodeConfig::regtest()
    .with_rpc("http://localhost:18884", "user", "password");
let client = RpcClient::new(config)?;

// Method 3: Quick URL-based setup
let client = RpcClient::from_url("http://localhost:18884", "user", "pass")?;

// Test connection
client.test_connection()?;
```

### Configuration File (musk.toml)

```toml
[network]
network = "regtest"  # or "testnet", "liquidv1"

[rpc]
url = "http://127.0.0.1:18884"
user = "user"
password = "password"

[chain]
genesis_hash = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
```

### Basic Contract Example

```rust
use musk::{Contract, Arguments, RpcClient, NodeConfig};
use musk::client::NodeClient;

// Connect to node
let client = RpcClient::new(NodeConfig::regtest())?;

// Load and compile a contract
let contract = Contract::from_file("my_contract.simf")?;
let compiled = contract.instantiate(Arguments::default())?;

// Generate an address (uses network-appropriate params)
let address = compiled.address(client.address_params());
println!("Contract address: {}", address);

// Fund the contract
let txid = client.send_to_address(&address, 100_000_000)?; // 1 BTC
client.generate_blocks(1)?; // Confirm (regtest only)
```

### Building Spending Transactions

```rust
use musk::{SpendBuilder, WitnessValues};

// Build a spending transaction
let mut builder = SpendBuilder::new(compiled, utxo)
    .genesis_hash(client.genesis_hash()?);

// Add outputs
builder.add_output_simple(destination, amount, asset);
builder.add_fee(3000, asset);

// Compute sighash for signature generation
let sighash = builder.sighash_all()?;

// Create witness values (with signatures)
let witness = WitnessValues::default();

// Finalize and broadcast
let tx = builder.finalize(witness)?;
let txid = client.broadcast(&tx)?;
```

### With Arguments and Witnesses

```rust
use musk::{Contract, Arguments, Value, WitnessName};
use std::collections::HashMap;

// Load contract with parameters
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

Musk is designed to be network-agnostic through the `NodeClient` trait:

```
┌─────────────────────────────────────────────────────────────┐
│                         Your App                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                          Musk                               │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Contract   │→ │CompiledContract│→ │   SpendBuilder   │   │
│  │  (.simf)    │  │  (Address)   │  │  (Transaction)   │   │
│  └─────────────┘  └──────────────┘  └──────────────────┘   │
│                                              │              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │               NodeClient trait                       │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────────┐    │   │
│  │  │ RpcClient │  │  (spray)  │  │ (your impl)   │    │   │
│  │  │ (built-in)│  │           │  │               │    │   │
│  │  └───────────┘  └───────────┘  └───────────────┘    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Elements/Liquid Node (RPC)                     │
│         regtest  │  testnet  │  liquidv1 (mainnet)          │
└─────────────────────────────────────────────────────────────┘
```

## Features

- `serde`: Enable serialization support (default)
- `rpc`: Enable RpcClient and config file support (default)

To use without RPC support:

```toml
[dependencies]
musk = { path = "../musk", default-features = false, features = ["serde"] }
```

## Network Support

| Network | Default Port | Address Params |
|---------|--------------|----------------|
| regtest | 18884 | `ELEMENTS` |
| testnet | 18892 | `LIQUID_TESTNET` |
| liquidv1 | 7041 | `LIQUID` |

## Examples

See the `examples/` directory:

- `basic_usage.rs` - Simple contract workflow
- `rpc_client.rs` - Connecting to nodes with RpcClient

Run examples:

```bash
cargo run --example basic_usage
cargo run --example rpc_client
```

## License

MIT OR Apache-2.0
