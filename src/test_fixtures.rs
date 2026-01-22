//! Test fixtures and constants for musk tests

#![allow(dead_code)] // Test fixtures may not all be used in every test

/// Simple program that always succeeds
pub const SIMPLE_PROGRAM: &str = "fn main() { assert!(true); }";

/// Simple program that always fails
pub const FAILING_PROGRAM: &str = "fn main() { assert!(false); }";

/// Program with a parameter
pub const PARAMETERIZED_PROGRAM: &str = r"
fn main() {
    let x: u32 = param::VALUE;
    assert!(jet::eq_32(x, 42));
}
";

/// Pay-to-public-key program for signature tests
pub const P2PK_PROGRAM: &str = r"
fn main() {
    let pk: Pubkey = param::PK;
    let sig: Signature = witness::SIG;
    assert!(jet::bip_0340_verify((pk, jet::sig_all_hash()), sig));
}
";

/// `OP_CAT` program from `SimplicityHL` examples
pub const CAT_PROGRAM: &str = r"
fn main() {
    let ab: u16 = <(u8, u8)>::into((0x10, 0x01));
    let c: u16 = 0x1001;
    assert!(jet::eq_16(ab, c));
    let ab: u8 = <(u4, u4)>::into((0b1011, 0b1101));
    let c: u8 = 0b10111101;
    assert!(jet::eq_8(ab, c));
}
";

/// Helper to create a dummy genesis hash for testing
#[must_use]
pub fn test_genesis_hash() -> elements::BlockHash {
    use elements::hashes::Hash;
    elements::BlockHash::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array([1u8; 32]))
}

/// Helper to create a dummy UTXO for testing
#[must_use]
pub fn test_utxo() -> crate::client::Utxo {
    use elements::hashes::Hash;
    use elements::issuance::AssetId;

    crate::client::Utxo {
        txid: elements::Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array(
            [2u8; 32],
        )),
        vout: 0,
        amount: 100_000_000,
        script_pubkey: elements::Script::new(),
        asset: elements::confidential::Asset::Explicit(
            AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
        ),
        amount_blinder: None,
        asset_blinder: None,
        amount_commitment: None,
        asset_commitment: None,
    }
}

/// Helper to create a test address
#[must_use]
pub fn test_address() -> elements::Address {
    // Create a simple P2WPKH address for testing
    use elements::bitcoin::PublicKey;
    use elements::AddressParams;
    use secp256k1::Secp256k1;

    let secp = Secp256k1::new();
    let secret_key = secp256k1::SecretKey::from_slice(&[1u8; 32]).expect("valid key");
    let secp_pubkey = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let bitcoin_pubkey = PublicKey::new(secp_pubkey);

    elements::Address::p2wpkh(&bitcoin_pubkey, None, &AddressParams::ELEMENTS)
}
