//! Musk - SDK for compiling, deploying, and spending Simplicity contracts
//!
//! This crate provides a high-level interface for working with Simplicity contracts
//! on Elements/Liquid networks. It wraps the SimplicityHL compiler and provides
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

pub mod address;
pub mod client;
pub mod contract;
pub mod error;
pub mod spend;
pub mod util;
pub mod witness;

// Re-export core types
pub use contract::{CompiledContract, Contract};
pub use error::{ContractError, SpendError};
pub use spend::SpendBuilder;
pub use client::NodeClient;

// Re-export SimplicityHL types for convenience
pub use simplicityhl::{Arguments, Parameters, Value, WitnessValues};
pub use simplicityhl::str::WitnessName;

// Re-export commonly used external types
pub use elements;
pub use elements::{Address, AddressParams, Transaction, Txid};

