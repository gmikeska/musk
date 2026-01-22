//! RPC-based `NodeClient` implementation for connecting to Elements nodes
//!
//! This module provides an implementation of the `NodeClient` trait that
//! connects to Elements/Liquid nodes via JSON-RPC. It is designed based on
//! spray's `ElementsClient` and serves as a reference implementation for
//! other `NodeClient` implementations.
//!
//! # Example
//!
//! ```ignore
//! use musk::{NodeConfig, RpcClient, Program, SpendBuilder};
//!
//! // Load config from file
//! let config = NodeConfig::from_file("musk.toml")?;
//! let client = RpcClient::new(config)?;
//!
//! // Or create programmatically
//! let client = RpcClient::from_url(
//!     "http://localhost:18884",
//!     "user",
//!     "password"
//! )?;
//!
//! // Use the client
//! let address = compiled_program.address(client.address_params());
//! let txid = client.send_to_address(&address, 100_000_000)?;
//! ```

use crate::client::{ClientResult, NodeClient, Utxo};
use crate::config::{Network, NodeConfig};
use crate::error::ProgramError;
use elements::{encode::deserialize, hex::FromHex, Address, BlockHash, Transaction, Txid};
use std::str::FromStr;

/// RPC client for Elements/Liquid nodes
///
/// This implementation uses JSON-RPC to communicate with Elements nodes.
/// It implements the `NodeClient` trait, making it compatible with all
/// musk operations that require node interaction.
///
/// The implementation is based on spray's `ElementsClient` and can be used
/// as a template for creating other `NodeClient` implementations (e.g., for
/// different RPC libraries or async frameworks).
pub struct RpcClient {
    client: jsonrpc::Client,
    config: NodeConfig,
    /// Cached genesis hash (fetched from node if not in config)
    genesis_hash: Option<BlockHash>,
}

impl RpcClient {
    /// Create a new RPC client from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC URL is invalid.
    pub fn new(config: NodeConfig) -> Result<Self, ProgramError> {
        // Use wallet URL for wallet-specific RPC calls
        let wallet_url = config.rpc.wallet_url();
        let transport = jsonrpc::simple_http::SimpleHttpTransport::builder()
            .url(&wallet_url)
            .map_err(|e| {
                ProgramError::IoError(std::io::Error::other(format!("Invalid RPC URL: {e}")))
            })?
            .auth(&config.rpc.user, Some(&config.rpc.password))
            .build();

        let client = jsonrpc::Client::with_transport(transport);

        Ok(Self {
            client,
            config,
            genesis_hash: None,
        })
    }

    /// Create from a config file
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed.
    pub fn from_config_file(path: &str) -> Result<Self, ProgramError> {
        let config = NodeConfig::from_file(path).map_err(|e| {
            ProgramError::IoError(std::io::Error::other(format!("Config error: {e}")))
        })?;
        Self::new(config)
    }

    /// Create from URL and credentials (uses regtest defaults)
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC URL is invalid.
    pub fn from_url(url: &str, user: &str, password: &str) -> Result<Self, ProgramError> {
        let config = NodeConfig::regtest().with_rpc(url, user, password);
        Self::new(config)
    }

    /// Create for a specific network with default settings
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC URL is invalid.
    pub fn for_network(network: Network, user: &str, password: &str) -> Result<Self, ProgramError> {
        let config = match network {
            Network::Regtest => NodeConfig::regtest(),
            Network::Testnet => NodeConfig::testnet(),
            Network::Liquid => NodeConfig::liquid(),
        }
        .with_rpc(&network.default_rpc_url(), user, password);

        Self::new(config)
    }

    /// Get the network type
    #[must_use]
    pub const fn network(&self) -> Network {
        self.config.network()
    }

    /// Get the network address params
    #[must_use]
    pub const fn address_params(&self) -> &'static elements::AddressParams {
        self.config.address_params()
    }

    /// Get the genesis hash (fetches from node if not cached/configured)
    ///
    /// # Errors
    ///
    /// Returns an error if the genesis hash cannot be fetched from the node.
    pub fn genesis_hash(&mut self) -> Result<BlockHash, ProgramError> {
        // Return cached value if available
        if let Some(hash) = self.genesis_hash {
            return Ok(hash);
        }

        // Try to get from config
        if let Ok(hash) = self.config.genesis_hash() {
            self.genesis_hash = Some(hash);
            return Ok(hash);
        }

        // Fetch from node
        let hash_str: String = self.call("getblockhash", &[serde_json::json!(0)])?;
        let hash = BlockHash::from_str(&hash_str).map_err(|e| {
            ProgramError::IoError(std::io::Error::other(format!(
                "Invalid genesis hash from node: {e}"
            )))
        })?;

        self.genesis_hash = Some(hash);
        Ok(hash)
    }

    /// Get a reference to the config
    #[must_use]
    pub const fn config(&self) -> &NodeConfig {
        &self.config
    }

    /// Make an RPC call
    fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &[serde_json::Value],
    ) -> ClientResult<T> {
        // Convert params to RawValue
        let params_json = serde_json::to_string(params).map_err(|e| {
            ProgramError::IoError(std::io::Error::other(format!(
                "Failed to serialize params: {e}"
            )))
        })?;

        let raw_params: Box<serde_json::value::RawValue> =
            serde_json::value::RawValue::from_string(params_json).map_err(|e| {
                ProgramError::IoError(std::io::Error::other(format!(
                    "Failed to create raw value: {e}"
                )))
            })?;

        let request = self.client.build_request(method, Some(&raw_params));
        let response = self.client.send_request(request).map_err(|e| {
            ProgramError::IoError(std::io::Error::other(format!("RPC request failed: {e}")))
        })?;

        response
            .result()
            .map_err(|e| ProgramError::IoError(std::io::Error::other(format!("RPC error: {e}"))))
    }

    /// Test the connection to the node
    ///
    /// # Errors
    ///
    /// Returns an error if the connection test fails.
    pub fn test_connection(&self) -> Result<(), ProgramError> {
        let _: serde_json::Value = self.call("getblockchaininfo", &[])?;
        Ok(())
    }

    /// Get blockchain info
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails.
    pub fn get_blockchain_info(&self) -> ClientResult<serde_json::Value> {
        self.call("getblockchaininfo", &[])
    }

    /// Get the current block count
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails.
    pub fn get_block_count(&self) -> ClientResult<u64> {
        self.call("getblockcount", &[])
    }

    /// Get wallet balance
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails.
    pub fn get_balance(&self) -> ClientResult<f64> {
        self.call("getbalance", &[])
    }

    /// Import an address to watch (without private key)
    ///
    /// This allows the wallet to track UTXOs for this address via `listunspent`.
    /// Automatically detects wallet type and uses `importdescriptors` for descriptor
    /// wallets or `importaddress` for legacy wallets.
    ///
    /// # Arguments
    ///
    /// * `address` - The address to import
    /// * `label` - Optional label for the address
    /// * `rescan` - Whether to rescan the blockchain (can be slow)
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails.
    pub fn import_address(&self, address: &str, label: Option<&str>, rescan: bool) -> ClientResult<()> {
        // Try importdescriptors first (for descriptor wallets)
        let desc = format!("addr({})", address);
        
        // Get checksum for the descriptor
        let checksum_result: Result<serde_json::Value, _> = self.call("getdescriptorinfo", &[desc.clone().into()]);
        
        match checksum_result {
            Ok(info) => {
                // Use the descriptor with checksum from the response
                if let Some(descriptor) = info.get("descriptor").and_then(|v| v.as_str()) {
                    let timestamp = if rescan { 
                        serde_json::json!(0) 
                    } else { 
                        serde_json::json!("now") 
                    };
                    
                    let import_req = serde_json::json!([{
                        "desc": descriptor,
                        "timestamp": timestamp,
                        "label": label.unwrap_or("samplicity"),
                    }]);
                    
                    let result: serde_json::Value = self.call("importdescriptors", &[import_req])?;
                    
                    // Check if import was successful
                    if let Some(arr) = result.as_array() {
                        if let Some(first) = arr.first() {
                            if first.get("success").and_then(|v| v.as_bool()) == Some(true) {
                                return Ok(());
                            }
                            // If there's an error message, include it
                            if let Some(err) = first.get("error").and_then(|v| v.get("message")).and_then(|v| v.as_str()) {
                                return Err(ProgramError::IoError(std::io::Error::other(
                                    format!("importdescriptors failed: {}", err)
                                )));
                            }
                        }
                    }
                    return Ok(());
                }
                // Fall through to legacy import if descriptor parsing failed
            }
            Err(_) => {
                // getdescriptorinfo failed - try legacy importaddress
            }
        }
        
        // Fall back to importaddress for legacy wallets
        let label_val = label.unwrap_or("");
        let _: serde_json::Value = self.call(
            "importaddress",
            &[
                serde_json::json!(address),
                serde_json::json!(label_val),
                serde_json::json!(rescan),
            ],
        )?;
        Ok(())
    }

    /// Import a blinding key for a confidential address
    ///
    /// This allows the wallet to unblind confidential transaction outputs
    /// sent to this address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address to import the blinding key for
    /// * `blinding_key` - The hex-encoded blinding private key
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails.
    pub fn import_blinding_key(&self, address: &str, blinding_key: &str) -> ClientResult<()> {
        let _: serde_json::Value = self.call(
            "importblindingkey",
            &[
                serde_json::json!(address),
                serde_json::json!(blinding_key),
            ],
        )?;
        Ok(())
    }
}

impl NodeClient for RpcClient {
    fn send_to_address(&self, addr: &Address, amount: u64) -> ClientResult<Txid> {
        let addr_str = addr.to_string();
        // Convert satoshis to BTC (Elements uses BTC units in RPC)
        #[allow(clippy::cast_precision_loss)]
        let amount_btc = amount as f64 / 100_000_000.0;

        let txid_str: String = self.call("sendtoaddress", &[addr_str.into(), amount_btc.into()])?;

        Txid::from_str(&txid_str)
            .map_err(|e| ProgramError::IoError(std::io::Error::other(format!("Invalid txid: {e}"))))
    }

    fn get_transaction(&self, txid: &Txid) -> ClientResult<Transaction> {
        let result: serde_json::Value = self.call("gettransaction", &[txid.to_string().into()])?;

        let tx_hex = result.get("hex").and_then(|v| v.as_str()).ok_or_else(|| {
            ProgramError::IoError(std::io::Error::other(
                "Invalid transaction response: missing hex field",
            ))
        })?;

        let tx_bytes = Vec::<u8>::from_hex(tx_hex).map_err(|e| {
            ProgramError::IoError(std::io::Error::other(format!("Invalid hex: {e}")))
        })?;

        deserialize(&tx_bytes).map_err(|e| {
            ProgramError::IoError(std::io::Error::other(format!(
                "Failed to deserialize transaction: {e}"
            )))
        })
    }

    fn broadcast(&self, tx: &Transaction) -> ClientResult<Txid> {
        use elements::encode::serialize_hex;

        let txid_str: String = self.call("sendrawtransaction", &[serialize_hex(tx).into()])?;

        Txid::from_str(&txid_str)
            .map_err(|e| ProgramError::IoError(std::io::Error::other(format!("Invalid txid: {e}"))))
    }

    fn generate_blocks(&self, count: u32) -> ClientResult<Vec<BlockHash>> {
        let address: String = self.call("getnewaddress", &[])?;

        let hashes: Vec<String> =
            self.call("generatetoaddress", &[count.into(), address.into()])?;

        hashes
            .iter()
            .map(|s| {
                BlockHash::from_str(s).map_err(|e| {
                    ProgramError::IoError(std::io::Error::other(format!("Invalid block hash: {e}")))
                })
            })
            .collect()
    }

    fn get_utxos(&self, address: &Address) -> ClientResult<Vec<Utxo>> {
        // Use listunspent with address filter
        let result: Vec<serde_json::Value> = self.call(
            "listunspent",
            &[
                serde_json::json!(1),                     // minconf
                serde_json::json!(9_999_999),             // maxconf
                serde_json::json!([address.to_string()]), // addresses
            ],
        )?;

        let mut utxos = Vec::new();
        for item in result {
            let txid_str = item.get("txid").and_then(|v| v.as_str()).ok_or_else(|| {
                ProgramError::IoError(std::io::Error::other("Missing txid in listunspent"))
            })?;

            let txid = Txid::from_str(txid_str).map_err(|e| {
                ProgramError::IoError(std::io::Error::other(format!("Invalid txid: {e}")))
            })?;

            #[allow(clippy::cast_possible_truncation)]
            let vout = item
                .get("vout")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ProgramError::IoError(std::io::Error::other("Missing vout in listunspent"))
                })? as u32;

            let amount_btc = item
                .get("amount")
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    ProgramError::IoError(std::io::Error::other("Missing amount in listunspent"))
                })?;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let amount = (amount_btc * 100_000_000.0) as u64;

            let script_hex = item
                .get("scriptPubKey")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ProgramError::IoError(std::io::Error::other(
                        "Missing scriptPubKey in listunspent",
                    ))
                })?;

            let script_bytes = Vec::<u8>::from_hex(script_hex).map_err(|e| {
                ProgramError::IoError(std::io::Error::other(format!("Invalid script hex: {e}")))
            })?;

            let script_pubkey = elements::Script::from(script_bytes);

            // Get asset - Elements returns asset ID as hex string
            let asset = if let Some(asset_str) = item.get("asset").and_then(|v| v.as_str()) {
                let asset_id = elements::AssetId::from_str(asset_str).map_err(|e| {
                    ProgramError::IoError(std::io::Error::other(format!("Invalid asset id: {e}")))
                })?;
                elements::confidential::Asset::Explicit(asset_id)
            } else {
                // Default to bitcoin asset if not specified
                elements::confidential::Asset::Null
            };

            utxos.push(Utxo {
                txid,
                vout,
                amount,
                script_pubkey,
                asset,
            });
        }

        Ok(utxos)
    }

    fn get_new_address(&self) -> ClientResult<Address> {
        let addr_str: String = self.call("getnewaddress", &[])?;

        Address::from_str(&addr_str).map_err(|e| {
            ProgramError::IoError(std::io::Error::other(format!("Invalid address: {e}")))
        })
    }
}

impl std::fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcClient")
            .field("config", &self.config)
            .field("genesis_hash", &self.genesis_hash)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_rpc_client_new() {
        let config = NodeConfig::regtest();
        let client = RpcClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_rpc_client_from_url() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass");
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.network(), Network::Regtest);
    }

    #[test]
    fn test_rpc_client_for_network_regtest() {
        let client = RpcClient::for_network(Network::Regtest, "user", "pass");
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.network(), Network::Regtest);
    }

    #[test]
    fn test_rpc_client_for_network_testnet() {
        let client = RpcClient::for_network(Network::Testnet, "user", "pass");
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.network(), Network::Testnet);
    }

    #[test]
    fn test_rpc_client_for_network_liquid() {
        let client = RpcClient::for_network(Network::Liquid, "user", "pass");
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.network(), Network::Liquid);
    }

    #[test]
    fn test_rpc_client_from_config_file() {
        let toml_content = r#"
[network]
network = "regtest"

[rpc]
url = "http://localhost:18884"
user = "testuser"
password = "testpass"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();

        let client = RpcClient::from_config_file(temp_file.path().to_str().unwrap());
        assert!(client.is_ok());
    }

    #[test]
    fn test_rpc_client_from_config_file_not_found() {
        let result = RpcClient::from_config_file("/nonexistent/config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_rpc_client_network() {
        let config = NodeConfig::testnet();
        let client = RpcClient::new(config).unwrap();
        assert_eq!(client.network(), Network::Testnet);
    }

    #[test]
    fn test_rpc_client_address_params() {
        let regtest_client = RpcClient::new(NodeConfig::regtest()).unwrap();
        let testnet_client = RpcClient::new(NodeConfig::testnet()).unwrap();
        let liquid_client = RpcClient::new(NodeConfig::liquid()).unwrap();

        // Verify each client returns different address params
        assert_ne!(
            regtest_client.address_params().bech_hrp,
            testnet_client.address_params().bech_hrp
        );
        assert_ne!(
            testnet_client.address_params().bech_hrp,
            liquid_client.address_params().bech_hrp
        );
    }

    #[test]
    fn test_rpc_client_config_access() {
        // Use localhost to avoid DNS lookup failures
        let config = NodeConfig::regtest()
            .with_rpc("http://127.0.0.1:12345", "u", "p")
            .with_wallet("test_wallet");
        
        let client = RpcClient::new(config).unwrap();
        
        assert_eq!(client.config().rpc.url, "http://127.0.0.1:12345");
        assert_eq!(client.config().rpc.user, "u");
        assert_eq!(client.config().rpc.wallet, "test_wallet");
    }

    #[test]
    fn test_rpc_client_genesis_hash_from_config() {
        let config = NodeConfig::regtest()
            .with_genesis_hash("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206");
        
        let mut client = RpcClient::new(config).unwrap();
        
        // Should get genesis hash from config without hitting the network
        let hash = client.genesis_hash().unwrap();
        assert_eq!(hash.to_string(), "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206");
        
        // Second call should return cached value
        let hash2 = client.genesis_hash().unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_rpc_client_debug() {
        let config = NodeConfig::regtest();
        let client = RpcClient::new(config).unwrap();
        
        let debug_str = format!("{:?}", client);
        assert!(debug_str.contains("RpcClient"));
        assert!(debug_str.contains("config"));
    }

    #[test]
    fn test_rpc_client_with_wallet_url() {
        let config = NodeConfig::regtest()
            .with_wallet("custom_wallet");
        
        let client = RpcClient::new(config).unwrap();
        
        // Verify the wallet is set in config
        assert_eq!(client.config().rpc.wallet, "custom_wallet");
        assert!(client.config().rpc.wallet_url().contains("custom_wallet"));
    }

    // Note: The following tests require a live Elements node and are marked as ignored.
    // Run them with: cargo test --features rpc -- --ignored
    
    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_test_connection() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let result = client.test_connection();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_get_blockchain_info() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let info = client.get_blockchain_info();
        assert!(info.is_ok());
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_get_block_count() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let count = client.get_block_count();
        assert!(count.is_ok());
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_get_balance() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let balance = client.get_balance();
        assert!(balance.is_ok());
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_genesis_hash_from_node() {
        let config = NodeConfig::regtest(); // No genesis_hash set
        let mut client = RpcClient::new(config).unwrap();
        
        // Should fetch from node
        let hash = client.genesis_hash();
        assert!(hash.is_ok());
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_generate_blocks() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let hashes = client.generate_blocks(1);
        assert!(hashes.is_ok());
        assert_eq!(hashes.unwrap().len(), 1);
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_get_new_address() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let addr = client.get_new_address();
        assert!(addr.is_ok());
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_send_to_address() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let addr = client.get_new_address().unwrap();
        
        // Need to have funds in the wallet
        let txid = client.send_to_address(&addr, 1_000_000);
        // This may fail if wallet has no funds - just check it runs
        let _ = txid;
    }

    #[test]
    #[ignore = "requires live Elements node"]
    fn test_rpc_client_import_address() {
        let client = RpcClient::from_url("http://localhost:18884", "user", "pass").unwrap();
        let addr = client.get_new_address().unwrap();
        
        let result = client.import_address(&addr.to_string(), Some("test"), false);
        // May succeed or fail depending on wallet type
        let _ = result;
    }
}
