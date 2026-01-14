//! Example: Using musk library for program operations
//!
//! This example shows how to use musk in a production application

use musk::{
    client::NodeClient, Arguments, Program, SpendBuilder, Value, WitnessName, WitnessValues,
};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Musk Library Usage Example\n");

    // 1. Load a program
    println!("1. Loading program...");
    let program = Program::from_source(
        r#"
        fn main() {
            // Simple program that always succeeds
            assert!(true);
        }
    "#,
    )?;
    println!("   ✓ Program loaded");

    // 2. Check parameters
    println!("\n2. Checking parameters...");
    let params = program.parameters();
    println!("   Parameters: {:?}", params);

    // 3. Compile the program
    println!("\n3. Compiling program...");
    let compiled = program.instantiate(Arguments::default())?;
    println!("   ✓ Program compiled");
    println!("   CMR: {}", compiled.cmr());

    // 4. Generate address
    println!("\n4. Generating address...");
    let address = compiled.address(&musk::elements::AddressParams::ELEMENTS);
    println!("   Address: {}", address);

    // 5. Show transaction building (conceptual)
    println!("\n5. Building transaction (conceptual)...");
    println!("   To spend from this program:");
    println!("   - Create a SpendBuilder with the compiled program and UTXO");
    println!("   - Add outputs and set lock_time/sequence as needed");
    println!("   - Compute sighash_all() for signature generation");
    println!("   - Generate witness values");
    println!("   - Finalize the transaction");

    Ok(())
}
