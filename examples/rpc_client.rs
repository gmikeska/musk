//! Example: Using musk's RpcClient to connect to Elements nodes
//!
//! This example demonstrates how to use the RpcClient to connect to
//! an Elements node and interact with Simplicity programs.
//!
//! Run with: cargo run --example rpc_client
//!
//! Prerequisites:
//!   - Elements node running (regtest mode)
//!   - RPC credentials configured

use musk::{Arguments, Program, NodeConfig, RpcClient, SpendBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Musk RpcClient Example\n");

    // ==========================================================================
    // Method 1: Load config from file
    // ==========================================================================
    println!("1. Loading config from file...");

    // Uncomment to load from file:
    // let client = RpcClient::from_config_file("musk.toml")?;

    // ==========================================================================
    // Method 2: Create config programmatically
    // ==========================================================================
    println!("2. Creating config programmatically...");

    let config = NodeConfig::regtest().with_rpc("http://127.0.0.1:18884", "user", "password");

    println!("   Network: {}", config.network());
    println!("   RPC URL: {}", config.rpc.url);

    // Try to connect (will fail if node isn't running)
    println!("\n3. Attempting to connect to node...");
    match RpcClient::new(config) {
        Ok(client) => {
            println!("   ✓ Client created");

            // Test connection
            match client.test_connection() {
                Ok(()) => {
                    println!("   ✓ Connection successful!");

                    // Get some info
                    if let Ok(count) = client.get_block_count() {
                        println!("   Block count: {}", count);
                    }
                    if let Ok(balance) = client.get_balance() {
                        println!("   Balance: {} BTC", balance);
                    }

                    // Demonstrate program workflow
                    demonstrate_program_workflow(&client)?;
                }
                Err(e) => {
                    println!("   ✗ Connection failed: {}", e);
                    println!("   Make sure Elements node is running");
                }
            }
        }
        Err(e) => {
            println!("   ✗ Failed to create client: {}", e);
        }
    }

    // ==========================================================================
    // Method 3: Network-specific shortcuts
    // ==========================================================================
    println!("\n4. Network shortcuts...");
    println!("   NodeConfig::regtest() - Default regtest config");
    println!("   NodeConfig::testnet() - Default testnet config");
    println!("   NodeConfig::liquid()  - Default Liquid mainnet config");

    // ==========================================================================
    // Method 4: Save config to file
    // ==========================================================================
    println!("\n5. Saving config to file...");
    let config = NodeConfig::regtest();
    match config.to_toml() {
        Ok(toml_str) => {
            println!("   Generated TOML:\n");
            for line in toml_str.lines().take(10) {
                println!("   {}", line);
            }
            println!("   ...\n");
        }
        Err(e) => {
            println!("   Failed to generate TOML: {}", e);
        }
    }

    Ok(())
}

fn demonstrate_program_workflow(client: &RpcClient) -> Result<(), Box<dyn std::error::Error>> {
    use musk::client::NodeClient;

    println!("\n6. Program workflow demonstration...");

    // Load a simple program
    let program_source = r#"
fn main() {
    assert!(true);
}
"#;

    let program = Program::from_source(program_source)?;
    println!("   ✓ Program loaded");

    let compiled = program.instantiate(Arguments::default())?;
    println!("   ✓ Program compiled");
    println!("   CMR: {}", compiled.cmr());

    // Generate address using client's network params
    let address = compiled.address(client.address_params());
    println!("   Address: {}", address);

    // Fund the address (requires wallet with funds)
    println!("\n   Funding program address...");
    match client.send_to_address(&address, 100_000_000) {
        // 1 BTC
        Ok(txid) => {
            println!("   ✓ Funded! txid: {}", txid);

            // Generate a block to confirm
            match client.generate_blocks(1) {
                Ok(_) => println!("   ✓ Block generated"),
                Err(e) => println!("   Note: Could not generate block: {}", e),
            }
        }
        Err(e) => {
            println!("   Note: Could not fund ({})", e);
            println!("   This is expected if wallet has no funds");
        }
    }

    Ok(())
}
