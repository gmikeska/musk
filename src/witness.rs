//! Witness utilities and signing helpers

use crate::util;
use simplicityhl::{Value, WitnessName, WitnessValues};
use std::collections::HashMap;

/// Builder for constructing witness values
pub struct WitnessBuilder {
    values: HashMap<WitnessName, Value>,
}

impl WitnessBuilder {
    /// Create a new witness builder
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Add a witness value
    pub fn with(mut self, name: &str, value: Value) -> Self {
        self.values.insert(WitnessName::from_str_unchecked(name), value);
        self
    }

    /// Add a signature witness (signs the given message with the secret key)
    pub fn with_signature(mut self, name: &str, secret_key: u32, message: [u8; 32]) -> Self {
        let signature = util::sign_schnorr(secret_key, message);
        self.values.insert(
            WitnessName::from_str_unchecked(name),
            Value::byte_array(signature),
        );
        self
    }

    /// Add a public key witness
    pub fn with_pubkey(mut self, name: &str, secret_key: u32) -> Self {
        let pubkey = util::xonly_public_key(secret_key);
        self.values.insert(
            WitnessName::from_str_unchecked(name),
            Value::u256(simplicityhl::num::U256::from_byte_array(pubkey)),
        );
        self
    }

    /// Build the witness values
    pub fn build(self) -> WitnessValues {
        WitnessValues::from(self.values)
    }
}

impl Default for WitnessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

