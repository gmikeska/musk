//! Musk - SDK for compiling, deploying, and spending Simplicity contracts
//!
//! This crate provides a high-level interface for working with Simplicity contracts
//! on Elements/Liquid networks. It wraps the `SimplicityHL` compiler and provides
//! utilities for address generation, transaction construction, and witness signing.
//!
//! # Example
//!
//! ```ignore
//! use musk::{Contract, Arguments};
//!
//! // Load and compile a contract
//! let contract = Contract::from_file("my_contract.simf")?;
//! let compiled = contract.instantiate(Arguments::default())?;
//!
//! // Generate an address
//! let address = compiled.address(&elements::AddressParams::ELEMENTS);
//!
//! // Build and sign a spending transaction
//! let builder = SpendBuilder::new(compiled, utxo);
//! let tx = builder.finalize(witness_values)?;
//! ```
//!
//! # Connecting to Nodes
//!
//! Use the `RpcClient` to connect to Elements/Liquid nodes:
//!
//! ```ignore
//! use musk::{NodeConfig, RpcClient};
//!
//! // From config file
//! let config = NodeConfig::from_file("musk.toml")?;
//! let client = RpcClient::new(config)?;
//!
//! // Or from URL
//! let client = RpcClient::from_url("http://localhost:18884", "user", "pass")?;
//!
//! // Use with contracts
//! let address = compiled.address(client.address_params());
//! let txid = client.send_to_address(&address, 100_000_000)?;
//! ```

pub mod address;
pub mod client;
#[cfg(feature = "rpc")]
pub mod config;
pub mod contract;
pub mod error;
#[cfg(feature = "rpc")]
pub mod rpc_client;
pub mod spend;
pub mod util;
pub mod witness;

#[cfg(test)]
mod mock_client;
#[cfg(test)]
mod test_fixtures;

// Re-export core types
pub use client::NodeClient;
pub use contract::{CompiledContract, Contract};
pub use error::{ContractError, SpendError};
pub use spend::SpendBuilder;

// Re-export config and RPC client when feature is enabled
#[cfg(feature = "rpc")]
pub use config::{ConfigError, Network, NodeConfig, RpcConfig};
#[cfg(feature = "rpc")]
pub use rpc_client::RpcClient;

// Re-export SimplicityHL types for convenience
pub use simplicityhl::str::WitnessName;
pub use simplicityhl::{Arguments, Parameters, Value, WitnessValues};

// Re-export commonly used external types
pub use elements;
pub use elements::{Address, AddressParams, Transaction, Txid};
