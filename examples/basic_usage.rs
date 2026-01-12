//! Example: Using musk library for contract operations
//!
//! This example shows how to use musk in a production application

use musk::{
    client::NodeClient,
    Contract, Arguments, SpendBuilder,
    WitnessValues, Value, WitnessName,
};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Musk Library Usage Example\n");

    // 1. Load a contract
    println!("1. Loading contract...");
    let contract = Contract::from_source(r#"
        fn main() {
            // Simple contract that always succeeds
            assert!(true);
        }
    "#)?;
    println!("   ✓ Contract loaded");

    // 2. Check parameters
    println!("\n2. Checking parameters...");
    let params = contract.parameters();
    println!("   Parameters: {:?}", params);

    // 3. Compile the contract
    println!("\n3. Compiling contract...");
    let compiled = contract.instantiate(Arguments::default())?;
    println!("   ✓ Contract compiled");
    println!("   CMR: {}", compiled.cmr());

    // 4. Generate address
    println!("\n4. Generating address...");
    let address = compiled.address(&musk::elements::AddressParams::ELEMENTS);
    println!("   Address: {}", address);

    // 5. Show transaction building (conceptual)
    println!("\n5. Building transaction (conceptual)...");
    println!("   To spend from this contract:");
    println!("   - Create a SpendBuilder with the compiled contract and UTXO");
    println!("   - Add outputs and set lock_time/sequence as needed");
    println!("   - Compute sighash_all() for signature generation");
    println!("   - Generate witness values");
    println!("   - Finalize the transaction");

    Ok(())
}

