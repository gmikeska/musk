//! RPC-based NodeClient implementation for connecting to Elements nodes
//!
//! This module provides an implementation of the `NodeClient` trait that
//! connects to Elements/Liquid nodes via JSON-RPC. It is designed based on
//! spray's `ElementsClient` and serves as a reference implementation for
//! other NodeClient implementations.
//!
//! # Example
//!
//! ```ignore
//! use musk::{NodeConfig, RpcClient, Contract, SpendBuilder};
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
//! let address = compiled_contract.address(client.address_params());
//! let txid = client.send_to_address(&address, 100_000_000)?;
//! ```

use crate::client::{ClientResult, NodeClient, Utxo};
use crate::config::{Network, NodeConfig};
use crate::error::ContractError;
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
    pub fn new(config: NodeConfig) -> Result<Self, ContractError> {
        let transport = jsonrpc::simple_http::SimpleHttpTransport::builder()
            .url(&config.rpc.url)
            .map_err(|e| {
                ContractError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid RPC URL: {}", e),
                ))
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
    pub fn from_config_file(path: &str) -> Result<Self, ContractError> {
        let config = NodeConfig::from_file(path).map_err(|e| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Config error: {}", e),
            ))
        })?;
        Self::new(config)
    }

    /// Create from URL and credentials (uses regtest defaults)
    pub fn from_url(url: &str, user: &str, password: &str) -> Result<Self, ContractError> {
        let config = NodeConfig::regtest().with_rpc(url, user, password);
        Self::new(config)
    }

    /// Create for a specific network with default settings
    pub fn for_network(
        network: Network,
        user: &str,
        password: &str,
    ) -> Result<Self, ContractError> {
        let config = match network {
            Network::Regtest => NodeConfig::regtest(),
            Network::Testnet => NodeConfig::testnet(),
            Network::Liquid => NodeConfig::liquid(),
        }
        .with_rpc(&network.default_rpc_url(), user, password);

        Self::new(config)
    }

    /// Get the network type
    pub fn network(&self) -> Network {
        self.config.network()
    }

    /// Get the network address params
    pub fn address_params(&self) -> &'static elements::AddressParams {
        self.config.address_params()
    }

    /// Get the genesis hash (fetches from node if not cached/configured)
    pub fn genesis_hash(&mut self) -> Result<BlockHash, ContractError> {
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
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid genesis hash from node: {}", e),
            ))
        })?;

        self.genesis_hash = Some(hash);
        Ok(hash)
    }

    /// Get a reference to the config
    pub fn config(&self) -> &NodeConfig {
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
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let raw_params: Box<serde_json::value::RawValue> =
            serde_json::value::RawValue::from_string(params_json).map_err(|e| {
                ContractError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create raw value: {}", e),
                ))
            })?;

        let request = self.client.build_request(method, Some(&raw_params));
        let response = self.client.send_request(request).map_err(|e| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("RPC request failed: {}", e),
            ))
        })?;

        response.result().map_err(|e| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("RPC error: {}", e),
            ))
        })
    }

    /// Test the connection to the node
    pub fn test_connection(&self) -> Result<(), ContractError> {
        let _: serde_json::Value = self.call("getblockchaininfo", &[])?;
        Ok(())
    }

    /// Get blockchain info
    pub fn get_blockchain_info(&self) -> ClientResult<serde_json::Value> {
        self.call("getblockchaininfo", &[])
    }

    /// Get the current block count
    pub fn get_block_count(&self) -> ClientResult<u64> {
        self.call("getblockcount", &[])
    }

    /// Get wallet balance
    pub fn get_balance(&self) -> ClientResult<f64> {
        self.call("getbalance", &[])
    }
}

impl NodeClient for RpcClient {
    fn send_to_address(&self, addr: &Address, amount: u64) -> ClientResult<Txid> {
        let addr_str = addr.to_string();
        // Convert satoshis to BTC (Elements uses BTC units in RPC)
        let amount_btc = amount as f64 / 100_000_000.0;

        let txid_str: String = self.call("sendtoaddress", &[addr_str.into(), amount_btc.into()])?;

        Txid::from_str(&txid_str).map_err(|e| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid txid: {}", e),
            ))
        })
    }

    fn get_transaction(&self, txid: &Txid) -> ClientResult<Transaction> {
        let result: serde_json::Value = self.call("gettransaction", &[txid.to_string().into()])?;

        let tx_hex = result.get("hex").and_then(|v| v.as_str()).ok_or_else(|| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid transaction response: missing hex field",
            ))
        })?;

        let tx_bytes = Vec::<u8>::from_hex(tx_hex).map_err(|e| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid hex: {}", e),
            ))
        })?;

        deserialize(&tx_bytes).map_err(|e| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to deserialize transaction: {}", e),
            ))
        })
    }

    fn broadcast(&self, tx: &Transaction) -> ClientResult<Txid> {
        use elements::encode::serialize_hex;

        let txid_str: String = self.call("sendrawtransaction", &[serialize_hex(tx).into()])?;

        Txid::from_str(&txid_str).map_err(|e| {
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid txid: {}", e),
            ))
        })
    }

    fn generate_blocks(&self, count: u32) -> ClientResult<Vec<BlockHash>> {
        let address: String = self.call("getnewaddress", &[])?;

        let hashes: Vec<String> =
            self.call("generatetoaddress", &[count.into(), address.into()])?;

        hashes
            .iter()
            .map(|s| {
                BlockHash::from_str(s).map_err(|e| {
                    ContractError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Invalid block hash: {}", e),
                    ))
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
                serde_json::json!(9999999),               // maxconf
                serde_json::json!([address.to_string()]), // addresses
            ],
        )?;

        let mut utxos = Vec::new();
        for item in result {
            let txid_str = item.get("txid").and_then(|v| v.as_str()).ok_or_else(|| {
                ContractError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Missing txid in listunspent",
                ))
            })?;

            let txid = Txid::from_str(txid_str).map_err(|e| {
                ContractError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid txid: {}", e),
                ))
            })?;

            let vout = item.get("vout").and_then(|v| v.as_u64()).ok_or_else(|| {
                ContractError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Missing vout in listunspent",
                ))
            })? as u32;

            let amount_btc = item.get("amount").and_then(|v| v.as_f64()).ok_or_else(|| {
                ContractError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Missing amount in listunspent",
                ))
            })?;
            let amount = (amount_btc * 100_000_000.0) as u64;

            let script_hex = item
                .get("scriptPubKey")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ContractError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Missing scriptPubKey in listunspent",
                    ))
                })?;

            let script_bytes = Vec::<u8>::from_hex(script_hex).map_err(|e| {
                ContractError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid script hex: {}", e),
                ))
            })?;

            let script_pubkey = elements::Script::from(script_bytes);

            // Get asset - Elements returns asset ID as hex string
            let asset = if let Some(asset_str) = item.get("asset").and_then(|v| v.as_str()) {
                let asset_id = elements::AssetId::from_str(asset_str).map_err(|e| {
                    ContractError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Invalid asset id: {}", e),
                    ))
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
            ContractError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid address: {}", e),
            ))
        })
    }
}

impl std::fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcClient")
            .field("config", &self.config)
            .field("genesis_hash", &self.genesis_hash)
            .finish()
    }
}
