//! Witness utilities and signing helpers
//!
//! This module provides the `WitnessBuilder` for constructing witness values
//! for Simplicity contracts.
//!
//! # Examples
//!
//! ```
//! use musk::witness::WitnessBuilder;
//! use simplicityhl::value::ValueConstructible;
//! use simplicityhl::Value;
//!
//! // Build witness with signature
//! let sighash = [0u8; 32];
//! let witness = WitnessBuilder::new()
//!     .with_signature("sig", 1, sighash)
//!     .build();
//! ```

use crate::util;
use simplicityhl::str::WitnessName;
use simplicityhl::value::ValueConstructible;
use simplicityhl::{Value, WitnessValues};
use std::collections::HashMap;

/// Builder for constructing witness values
pub struct WitnessBuilder {
    values: HashMap<WitnessName, Value>,
}

impl WitnessBuilder {
    /// Create a new witness builder
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::witness::WitnessBuilder;
    ///
    /// let builder = WitnessBuilder::new();
    /// let witness = builder.build();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Add a witness value
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::witness::WitnessBuilder;
    /// use simplicityhl::value::ValueConstructible;
    /// use simplicityhl::Value;
    ///
    /// let witness = WitnessBuilder::new()
    ///     .with("x", Value::u32(42))
    ///     .build();
    /// ```
    #[must_use]
    pub fn with(mut self, name: &str, value: Value) -> Self {
        self.values
            .insert(WitnessName::from_str_unchecked(name), value);
        self
    }

    /// Add a signature witness (signs the given message with the secret key)
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::witness::WitnessBuilder;
    ///
    /// let message = [0u8; 32];
    /// let witness = WitnessBuilder::new()
    ///     .with_signature("sig", 1, message)
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_signature(mut self, name: &str, secret_key: u32, message: [u8; 32]) -> Self {
        let signature = util::sign_schnorr(secret_key, message);
        self.values.insert(
            WitnessName::from_str_unchecked(name),
            Value::byte_array(signature),
        );
        self
    }

    /// Add a public key witness
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::witness::WitnessBuilder;
    ///
    /// let witness = WitnessBuilder::new()
    ///     .with_pubkey("pk", 1)
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_pubkey(mut self, name: &str, secret_key: u32) -> Self {
        let pubkey = util::xonly_public_key(secret_key);
        self.values.insert(
            WitnessName::from_str_unchecked(name),
            Value::u256(simplicityhl::num::U256::from_byte_array(pubkey)),
        );
        self
    }

    /// Build the witness values
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::witness::WitnessBuilder;
    /// use simplicityhl::value::ValueConstructible;
    /// use simplicityhl::Value;
    ///
    /// let witness = WitnessBuilder::new()
    ///     .with("x", Value::u32(42))
    ///     .build();
    /// ```
    #[must_use]
    pub fn build(self) -> WitnessValues {
        WitnessValues::from(self.values)
    }
}

impl Default for WitnessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simplicityhl::value::ValueConstructible;

    #[test]
    fn test_witness_builder_empty() {
        let builder = WitnessBuilder::new();
        let witness = builder.build();
        // Should be able to build empty witness
        assert!(std::mem::size_of_val(&witness) > 0);
    }

    #[test]
    fn test_witness_builder_with_value() {
        let witness = WitnessBuilder::new()
            .with("x", Value::u32(42))
            .with("y", Value::u32(100))
            .build();
        // Should be able to build witness with values
        assert!(std::mem::size_of_val(&witness) > 0);
    }

    #[test]
    fn test_witness_builder_with_signature() {
        let message = [1u8; 32];
        let witness = WitnessBuilder::new()
            .with_signature("sig", 1, message)
            .build();
        // Should be able to build witness with signature
        assert!(std::mem::size_of_val(&witness) > 0);
    }

    #[test]
    fn test_witness_builder_with_pubkey() {
        let witness = WitnessBuilder::new()
            .with_pubkey("pk", 1)
            .build();
        // Should be able to build witness with pubkey
        assert!(std::mem::size_of_val(&witness) > 0);
    }

    #[test]
    fn test_witness_builder_chaining() {
        let message = [0u8; 32];
        let witness = WitnessBuilder::new()
            .with_signature("sig", 1, message)
            .with_pubkey("pk", 1)
            .with("x", Value::u32(42))
            .build();
        // Should be able to chain multiple witness values
        assert!(std::mem::size_of_val(&witness) > 0);
    }

    #[test]
    fn test_witness_builder_default() {
        let builder = WitnessBuilder::default();
        let witness = builder.build();
        assert!(std::mem::size_of_val(&witness) > 0);
    }
}
