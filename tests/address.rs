//! Unit tests for address generation and taproot utilities

use musk::{Arguments, Contract};

#[test]
fn test_create_taproot_info() {
    // Compile a simple contract
    let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    let compiled = contract.instantiate(Arguments::default()).unwrap();

    // Get taproot info
    let taproot_info = compiled.taproot_info();

    // Verify internal key exists
    let internal_key = taproot_info.internal_key();
    assert_eq!(internal_key.serialize().len(), 32);

    // Verify merkle root exists
    let merkle_root = taproot_info.merkle_root();
    assert!(merkle_root.is_some());
}

#[test]
fn test_taproot_info_deterministic() {
    let source = "fn main() { assert!(true); }";

    // Compile same contract twice
    let contract1 = Contract::from_source(source).unwrap();
    let compiled1 = contract1.instantiate(Arguments::default()).unwrap();

    let contract2 = Contract::from_source(source).unwrap();
    let compiled2 = contract2.instantiate(Arguments::default()).unwrap();

    // Taproot info should be identical
    let info1 = compiled1.taproot_info();
    let info2 = compiled2.taproot_info();

    assert_eq!(info1.internal_key(), info2.internal_key());
    assert_eq!(info1.merkle_root(), info2.merkle_root());
}

#[test]
fn test_taproot_address_format() {
    let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    let compiled = contract.instantiate(Arguments::default()).unwrap();

    // Test regtest address
    let regtest_addr = compiled.address(&musk::elements::AddressParams::ELEMENTS);
    assert!(
        regtest_addr.to_string().starts_with("ert1p"),
        "Regtest taproot address should start with ert1p"
    );

    // Test liquid mainnet address
    let liquid_addr = compiled.address(&musk::elements::AddressParams::LIQUID);
    assert!(
        liquid_addr.to_string().starts_with("ex1p") || liquid_addr.to_string().starts_with("lq1p"),
        "Liquid taproot address should start with ex1p or lq1p"
    );

    // Test liquid testnet address
    let testnet_addr = compiled.address(&musk::elements::AddressParams::LIQUID_TESTNET);
    assert!(
        testnet_addr.to_string().starts_with("tex1p")
            || testnet_addr.to_string().starts_with("tlq1p"),
        "Testnet taproot address should start with tex1p or tlq1p"
    );
}

#[test]
fn test_script_version() {
    let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    let compiled = contract.instantiate(Arguments::default()).unwrap();

    let (script, leaf_version) = compiled.script_version();

    // Script should contain the CMR (32 bytes)
    assert_eq!(script.len(), 32, "Script should be 32 bytes (CMR)");

    // Verify script matches CMR
    let cmr = compiled.cmr();
    assert_eq!(script.as_bytes(), cmr.as_ref());

    // Leaf version should be the Simplicity leaf version
    assert_eq!(leaf_version, simplicityhl::simplicity::leaf_version());
}

#[test]
fn test_control_block_exists() {
    let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    let compiled = contract.instantiate(Arguments::default()).unwrap();

    let taproot_info = compiled.taproot_info();
    let (script, version) = compiled.script_version();

    // Should be able to get control block for our script
    let control_block = taproot_info.control_block(&(script, version));
    assert!(
        control_block.is_some(),
        "Control block should exist for the contract script"
    );
}

#[test]
fn test_different_contracts_different_addresses() {
    let contract1 = Contract::from_source("fn main() { assert!(true); }").unwrap();
    let compiled1 = contract1.instantiate(Arguments::default()).unwrap();

    // Different contract source
    let contract2 =
        Contract::from_source("fn main() { let x: u32 = 1; assert!(jet::eq_32(x, 1)); }").unwrap();
    let compiled2 = contract2.instantiate(Arguments::default()).unwrap();

    let addr1 = compiled1.address(&musk::elements::AddressParams::ELEMENTS);
    let addr2 = compiled2.address(&musk::elements::AddressParams::ELEMENTS);

    // Different contracts should produce different addresses
    assert_ne!(
        addr1, addr2,
        "Different contracts should have different addresses"
    );
}
