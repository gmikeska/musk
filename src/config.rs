//! Configuration for connecting to Elements nodes
//!
//! This module provides a configuration system for connecting musk to
//! Elements/Liquid nodes via RPC.
//!
//! # Example Configuration File (musk.toml)
//!
//! ```toml
//! [network]
//! network = "regtest"
//!
//! [rpc]
//! url = "http://127.0.0.1:18884"
//! user = "user"
//! password = "password"
//!
//! [chain]
//! genesis_hash = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Network type for Elements/Liquid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[default]
    Regtest,
    Testnet,
    #[serde(rename = "liquidv1")]
    Liquid,
}

impl Network {
    /// Get the default RPC port for this network
    pub fn default_rpc_port(&self) -> u16 {
        match self {
            Network::Regtest => 18884,
            Network::Testnet => 18892,
            Network::Liquid => 7041,
        }
    }

    /// Get the address params for this network
    pub fn address_params(&self) -> &'static elements::AddressParams {
        match self {
            Network::Regtest => &elements::AddressParams::ELEMENTS,
            Network::Testnet => &elements::AddressParams::LIQUID_TESTNET,
            Network::Liquid => &elements::AddressParams::LIQUID,
        }
    }

    /// Get the default RPC URL for this network
    pub fn default_rpc_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.default_rpc_port())
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Regtest => write!(f, "regtest"),
            Network::Testnet => write!(f, "testnet"),
            Network::Liquid => write!(f, "liquidv1"),
        }
    }
}

/// RPC connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// RPC URL (e.g., "http://127.0.0.1:18884")
    pub url: String,
    /// RPC username
    pub user: String,
    /// RPC password
    pub password: String,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            url: "http://127.0.0.1:18884".to_string(),
            user: "user".to_string(),
            password: "password".to_string(),
        }
    }
}

impl RpcConfig {
    /// Create RPC config for a specific network with default settings
    pub fn for_network(network: Network) -> Self {
        Self {
            url: network.default_rpc_url(),
            ..Default::default()
        }
    }
}

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainConfig {
    /// Genesis block hash (required for sighash computation)
    /// If not provided, will be fetched from the node
    pub genesis_hash: Option<String>,
}

/// Network configuration wrapper (for TOML structure)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct NetworkWrapper {
    network: Network,
}

/// Complete node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Network selection
    #[serde(default, rename = "network")]
    network_wrapper: NetworkWrapper,
    /// RPC connection settings
    #[serde(default)]
    pub rpc: RpcConfig,
    /// Chain-specific settings
    #[serde(default)]
    pub chain: ChainConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self::regtest()
    }
}

impl NodeConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_toml(&contents)
    }

    /// Parse configuration from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml_str).map_err(ConfigError::Parse)
    }

    /// Serialize configuration to TOML string
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(ConfigError::Serialize)
    }

    /// Save configuration to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let toml_str = self.to_toml()?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    /// Get the network type
    pub fn network(&self) -> Network {
        self.network_wrapper.network
    }

    /// Set the network type
    pub fn set_network(&mut self, network: Network) {
        self.network_wrapper.network = network;
    }

    /// Get the genesis hash as BlockHash
    pub fn genesis_hash(&self) -> Result<elements::BlockHash, ConfigError> {
        use std::str::FromStr;

        let hash_str = self
            .chain
            .genesis_hash
            .as_ref()
            .ok_or(ConfigError::MissingGenesisHash)?;

        elements::BlockHash::from_str(hash_str)
            .map_err(|e| ConfigError::InvalidGenesisHash(e.to_string()))
    }

    /// Get address params for the configured network
    pub fn address_params(&self) -> &'static elements::AddressParams {
        self.network().address_params()
    }

    /// Create a default config for regtest
    pub fn regtest() -> Self {
        Self {
            network_wrapper: NetworkWrapper {
                network: Network::Regtest,
            },
            rpc: RpcConfig::for_network(Network::Regtest),
            chain: ChainConfig::default(),
        }
    }

    /// Create a default config for testnet
    pub fn testnet() -> Self {
        Self {
            network_wrapper: NetworkWrapper {
                network: Network::Testnet,
            },
            rpc: RpcConfig::for_network(Network::Testnet),
            chain: ChainConfig::default(),
        }
    }

    /// Create a default config for Liquid mainnet
    pub fn liquid() -> Self {
        Self {
            network_wrapper: NetworkWrapper {
                network: Network::Liquid,
            },
            rpc: RpcConfig::for_network(Network::Liquid),
            chain: ChainConfig::default(),
        }
    }

    /// Create config with custom RPC settings
    pub fn with_rpc(mut self, url: &str, user: &str, password: &str) -> Self {
        self.rpc = RpcConfig {
            url: url.to_string(),
            user: user.to_string(),
            password: password.to_string(),
        };
        self
    }

    /// Set the genesis hash
    pub fn with_genesis_hash(mut self, hash: &str) -> Self {
        self.chain.genesis_hash = Some(hash.to_string());
        self
    }
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("Missing genesis hash in config (required for transaction signing)")]
    MissingGenesisHash,

    #[error("Invalid genesis hash: {0}")]
    InvalidGenesisHash(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.network(), Network::Regtest);
        assert_eq!(config.rpc.url, "http://127.0.0.1:18884");
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
[network]
network = "testnet"

[rpc]
url = "http://localhost:18892"
user = "myuser"
password = "mypass"

[chain]
genesis_hash = "abc123"
"#;
        let config = NodeConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.network(), Network::Testnet);
        assert_eq!(config.rpc.user, "myuser");
        assert_eq!(config.chain.genesis_hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_network_params() {
        assert_eq!(Network::Regtest.default_rpc_port(), 18884);
        assert_eq!(Network::Testnet.default_rpc_port(), 18892);
        assert_eq!(Network::Liquid.default_rpc_port(), 7041);
    }
}

