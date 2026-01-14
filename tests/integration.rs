//! Integration tests for musk

use musk::{Arguments, Program, WitnessValues};

#[test]
fn test_simple_program_lifecycle() {
    // Compile a simple program
    let program = Program::from_source("fn main() { assert!(true); }").unwrap();

    // Instantiate with no arguments
    let compiled = program.instantiate(Arguments::default()).unwrap();

    // Generate address
    let address = compiled.address(&elements::AddressParams::ELEMENTS);
    assert!(address.to_string().starts_with("ert1p"));

    // Get CMR
    let cmr = compiled.cmr();
    assert_eq!(cmr.as_ref().len(), 32);

    // Satisfy with empty witness
    let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();

    // Encode for transaction
    let (program, _witness) = satisfied.encode();
    assert!(!program.is_empty());
}

#[test]
fn test_program_determinism() {
    // Same source should produce same CMR
    let source = "fn main() { assert!(true); }";

    let program1 = Program::from_source(source).unwrap();
    let compiled1 = program1.instantiate(Arguments::default()).unwrap();

    let program2 = Program::from_source(source).unwrap();
    let compiled2 = program2.instantiate(Arguments::default()).unwrap();

    assert_eq!(compiled1.cmr(), compiled2.cmr());

    // Same address params should produce same address
    let addr1 = compiled1.address(&elements::AddressParams::ELEMENTS);
    let addr2 = compiled2.address(&elements::AddressParams::ELEMENTS);
    assert_eq!(addr1, addr2);
}

#[test]
fn test_witness_building() {
    use musk::witness::WitnessBuilder;
    use simplicityhl::value::ValueConstructible;
    use simplicityhl::Value;

    // Build witness with multiple values
    let witness = WitnessBuilder::new()
        .with("x", Value::u32(42))
        .with("y", Value::u32(100))
        .build();

    // Should be able to use witness
    assert!(std::mem::size_of_val(&witness) > 0);
}

#[test]
fn test_signature_witness() {
    use musk::witness::WitnessBuilder;

    // Build witness with signature
    let sighash = [1u8; 32];
    let witness = WitnessBuilder::new()
        .with_signature("sig", 1, sighash)
        .with_pubkey("pk", 1)
        .build();

    assert!(std::mem::size_of_val(&witness) > 0);
}

#[cfg(feature = "rpc")]
#[test]
fn test_network_config() {
    use musk::{Network, NodeConfig};

    // Test network defaults
    assert_eq!(Network::Regtest.default_rpc_port(), 18884);
    assert_eq!(Network::Testnet.default_rpc_port(), 18892);
    assert_eq!(Network::Liquid.default_rpc_port(), 7041);

    // Test config creation
    let config = NodeConfig::regtest();
    assert_eq!(config.network(), Network::Regtest);

    let config = NodeConfig::testnet();
    assert_eq!(config.network(), Network::Testnet);
}

#[test]
fn test_cryptographic_utilities() {
    use musk::util::{keypair_from_u32, sign_schnorr, xonly_public_key};

    // Test key generation
    let keypair = keypair_from_u32(42);
    assert_eq!(keypair.x_only_public_key().0.serialize().len(), 32);

    // Test public key extraction
    let pubkey = xonly_public_key(42);
    assert_eq!(pubkey.len(), 32);

    // Test signing
    let message = [0u8; 32];
    let signature = sign_schnorr(42, message);
    assert_eq!(signature.len(), 64);
}
