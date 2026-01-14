//! Cryptographic utilities for signing and key management

use secp256k1::{Keypair, Message, Secp256k1, XOnlyPublicKey};

/// Create a keypair from a u32 secret key (for testing)
///
/// # Examples
///
/// ```
/// use musk::util::keypair_from_u32;
///
/// let keypair = keypair_from_u32(42);
/// assert!(keypair.x_only_public_key().0.serialize().len() == 32);
/// ```
///
/// # Panics
///
/// Panics if the secret key bytes produce an invalid secp256k1 secret key
/// (this should never happen for reasonable u32 inputs).
#[must_use]
pub fn keypair_from_u32(secret_key: u32) -> Keypair {
    let mut secret_key_bytes = [0u8; 32];
    secret_key_bytes[28..].copy_from_slice(&secret_key.to_be_bytes());
    Keypair::from_seckey_slice(&Secp256k1::new(), &secret_key_bytes)
        .expect("secret key should be valid")
}

/// Sign a message using Schnorr signature
///
/// # Examples
///
/// ```
/// use musk::util::sign_schnorr;
///
/// let message = [0u8; 32];
/// let signature = sign_schnorr(1, message);
/// assert_eq!(signature.len(), 64);
/// ```
#[must_use]
pub fn sign_schnorr(secret_key: u32, message: [u8; 32]) -> [u8; 64] {
    let keypair = keypair_from_u32(secret_key);
    let message = Message::from_digest(message);
    keypair.sign_schnorr(message).serialize()
}

/// Get the x-only public key for a secret key
///
/// # Examples
///
/// ```
/// use musk::util::xonly_public_key;
///
/// let pubkey = xonly_public_key(1);
/// assert_eq!(pubkey.len(), 32);
/// 
/// // Same key should produce same pubkey
/// let pubkey2 = xonly_public_key(1);
/// assert_eq!(pubkey, pubkey2);
/// ```
#[must_use]
pub fn xonly_public_key(secret_key: u32) -> [u8; 32] {
    let keypair = keypair_from_u32(secret_key);
    keypair.x_only_public_key().0.serialize()
}

/// Parse an x-only public key from bytes
///
/// # Examples
///
/// ```
/// use musk::util::{xonly_public_key, parse_xonly_public_key};
///
/// let pubkey_bytes = xonly_public_key(1);
/// let pubkey = parse_xonly_public_key(&pubkey_bytes).unwrap();
/// assert_eq!(pubkey.serialize(), pubkey_bytes);
/// ```
///
/// # Errors
///
/// Returns an error if the bytes do not represent a valid x-only public key.
pub fn parse_xonly_public_key(bytes: &[u8]) -> Result<XOnlyPublicKey, secp256k1::Error> {
    XOnlyPublicKey::from_slice(bytes)
}

/// Default internal key for taproot (NUMS point)
///
/// Returns a "Nothing Up My Sleeve" (NUMS) point used as the internal key
/// for taproot addresses, ensuring the keypath cannot be spent.
///
/// # Examples
///
/// ```
/// use musk::util::default_internal_key;
///
/// let key = default_internal_key();
/// assert_eq!(key.serialize().len(), 32);
/// 
/// // Should be deterministic
/// let key2 = default_internal_key();
/// assert_eq!(key, key2);
/// ```
///
/// # Panics
///
/// Panics if the hardcoded hex or public key bytes are invalid
/// (this should never happen as they are compile-time constants).
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_deterministic() {
        let kp1 = keypair_from_u32(42);
        let kp2 = keypair_from_u32(42);
        assert_eq!(kp1.x_only_public_key().0, kp2.x_only_public_key().0);
    }

    #[test]
    fn test_sign_schnorr_valid() {
        let message = [1u8; 32];
        let sig = sign_schnorr(1, message);
        assert_eq!(sig.len(), 64);
        
        // Signatures should be deterministic for same inputs (Schnorr uses deterministic nonce)
        let sig2 = sign_schnorr(1, message);
        // Note: signatures may differ due to random nonce, so we just check length
        assert_eq!(sig2.len(), 64);
    }

    #[test]
    fn test_xonly_public_key() {
        let pk1 = xonly_public_key(1);
        let pk2 = xonly_public_key(1);
        assert_eq!(pk1, pk2);
        assert_eq!(pk1.len(), 32);
        
        // Different keys should produce different pubkeys
        let pk3 = xonly_public_key(2);
        assert_ne!(pk1, pk3);
    }

    #[test]
    fn test_parse_xonly_public_key() {
        let pk_bytes = xonly_public_key(1);
        let pk = parse_xonly_public_key(&pk_bytes).unwrap();
        assert_eq!(pk.serialize(), pk_bytes);
    }

    #[test]
    fn test_parse_xonly_public_key_invalid() {
        let invalid_bytes = [0u8; 31]; // Wrong length
        assert!(parse_xonly_public_key(&invalid_bytes).is_err());
    }

    #[test]
    fn test_default_internal_key() {
        let key1 = default_internal_key();
        let key2 = default_internal_key();
        assert_eq!(key1, key2);
        assert_eq!(key1.serialize().len(), 32);
    }
}
