//! Cryptographic utilities for signing and key management

use secp256k1::{Keypair, Message, Secp256k1, XOnlyPublicKey};

/// Create a keypair from a u32 secret key (for testing)
pub fn keypair_from_u32(secret_key: u32) -> Keypair {
    let mut secret_key_bytes = [0u8; 32];
    secret_key_bytes[28..].copy_from_slice(&secret_key.to_be_bytes());
    Keypair::from_seckey_slice(&Secp256k1::new(), &secret_key_bytes)
        .expect("secret key should be valid")
}

/// Sign a message using Schnorr signature
pub fn sign_schnorr(secret_key: u32, message: [u8; 32]) -> [u8; 64] {
    let keypair = keypair_from_u32(secret_key);
    let message = Message::from_digest(message);
    keypair.sign_schnorr(message).serialize()
}

/// Get the x-only public key for a secret key
pub fn xonly_public_key(secret_key: u32) -> [u8; 32] {
    let keypair = keypair_from_u32(secret_key);
    keypair.x_only_public_key().0.serialize()
}

/// Parse an x-only public key from bytes
pub fn parse_xonly_public_key(bytes: &[u8]) -> Result<XOnlyPublicKey, secp256k1::Error> {
    XOnlyPublicKey::from_slice(bytes)
}

/// Default internal key for taproot (NUMS point)
pub fn default_internal_key() -> XOnlyPublicKey {
    XOnlyPublicKey::from_slice(
        &hex::decode("50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0")
            .expect("valid hex"),
    )
    .expect("valid xonly pubkey")
}

// Add hex dependency for default_internal_key
#[doc(hidden)]
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}
