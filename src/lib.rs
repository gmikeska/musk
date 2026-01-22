//! Musk - SDK for compiling, deploying, and spending Simplicity programs
//!
//! This crate provides a high-level interface for working with Simplicity programs
//! on Elements/Liquid networks. It wraps the `SimplicityHL` compiler and provides
//! utilities for address generation, transaction construction, and witness signing.
//!
//! # Example
//!
//! ```ignore
//! use musk::{Program, Arguments};
//!
//! // Load and compile a program
//! let program = Program::from_file("my_program.simf")?;
//! let compiled = program.instantiate(Arguments::default())?;
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
//! // Use with programs
//! let address = compiled.address(client.address_params());
//! let txid = client.send_to_address(&address, 100_000_000)?;
//! ```

pub mod address;
pub mod client;
#[cfg(feature = "rpc")]
pub mod config;
pub mod error;
pub mod program;
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
pub use error::{ProgramError, SpendError};
pub use program::{AddressType, InstantiatedProgram, Program, SatisfiedProgram};
pub use spend::SpendBuilder;

// Re-export config and RPC client when feature is enabled
#[cfg(feature = "rpc")]
pub use config::{ConfigError, Network, NodeConfig, RpcConfig};
#[cfg(feature = "rpc")]
pub use rpc_client::RpcClient;

// Re-export SimplicityHL types for convenience
pub use simplicityhl::str::WitnessName;
pub use simplicityhl::value::ValueConstructible;
pub use simplicityhl::{Arguments, Parameters, Value, WitnessValues};

// Re-export simplicityhl for advanced usage
pub use simplicityhl;

// Re-export commonly used external types
pub use elements;
pub use elements::{Address, AddressParams, Transaction, Txid};
