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
    #[must_use]
    pub const fn default_rpc_port(self) -> u16 {
        match self {
            Self::Regtest => 18884,
            Self::Testnet => 18892,
            Self::Liquid => 7041,
        }
    }

    /// Get the address params for this network
    #[must_use]
    pub const fn address_params(self) -> &'static elements::AddressParams {
        match self {
            Self::Regtest => &elements::AddressParams::ELEMENTS,
            Self::Testnet => &elements::AddressParams::LIQUID_TESTNET,
            Self::Liquid => &elements::AddressParams::LIQUID,
        }
    }

    /// Get the default RPC URL for this network
    #[must_use]
    pub fn default_rpc_url(self) -> String {
        format!("http://127.0.0.1:{}", self.default_rpc_port())
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regtest => write!(f, "regtest"),
            Self::Testnet => write!(f, "testnet"),
            Self::Liquid => write!(f, "liquidv1"),
        }
    }
}

/// RPC connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// RPC URL (e.g., `http://127.0.0.1:18884`)
    pub url: String,
    /// RPC username
    pub user: String,
    /// RPC password
    pub password: String,
    /// Wallet name (defaults to "musk" if not specified)
    #[serde(default = "default_wallet_name")]
    pub wallet: String,
}

fn default_wallet_name() -> String {
    "musk".to_string()
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            url: "http://127.0.0.1:18884".to_string(),
            user: "user".to_string(),
            password: "password".to_string(),
            wallet: default_wallet_name(),
        }
    }
}

impl RpcConfig {
    /// Get the RPC URL with wallet path appended
    ///
    /// Elements RPC uses `/wallet/<name>` for wallet-specific operations
    #[must_use]
    pub fn wallet_url(&self) -> String {
        format!("{}/wallet/{}", self.url.trim_end_matches('/'), self.wallet)
    }
}

impl RpcConfig {
    /// Create RPC config for a specific network with default settings
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_toml(&contents)
    }

    /// Parse configuration from TOML string
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is invalid.
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml_str).map_err(ConfigError::Parse)
    }

    /// Serialize configuration to TOML string
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(ConfigError::Serialize)
    }

    /// Save configuration to a file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let toml_str = self.to_toml()?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    /// Get the network type
    #[must_use]
    pub const fn network(&self) -> Network {
        self.network_wrapper.network
    }

    /// Set the network type
    pub fn set_network(&mut self, network: Network) {
        self.network_wrapper.network = network;
    }

    /// Get the genesis hash as `BlockHash`
    ///
    /// # Errors
    ///
    /// Returns an error if the genesis hash is missing or invalid.
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
    #[must_use]
    pub const fn address_params(&self) -> &'static elements::AddressParams {
        self.network().address_params()
    }

    /// Create a default config for regtest
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn liquid() -> Self {
        Self {
            network_wrapper: NetworkWrapper {
                network: Network::Liquid,
            },
            rpc: RpcConfig::for_network(Network::Liquid),
            chain: ChainConfig::default(),
        }
    }

    /// Create config with custom RPC settings (preserves existing wallet name)
    #[must_use]
    pub fn with_rpc(mut self, url: &str, user: &str, password: &str) -> Self {
        self.rpc.url = url.to_string();
        self.rpc.user = user.to_string();
        self.rpc.password = password.to_string();
        self
    }

    /// Set the wallet name
    #[must_use]
    pub fn with_wallet(mut self, wallet: &str) -> Self {
        self.rpc.wallet = wallet.to_string();
        self
    }

    /// Set the genesis hash
    #[must_use]
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.network(), Network::Regtest);
        assert_eq!(config.rpc.url, "http://127.0.0.1:18884");
        assert_eq!(config.rpc.wallet, "musk");
    }

    #[test]
    fn test_wallet_url() {
        let config = NodeConfig::default();
        assert_eq!(
            config.rpc.wallet_url(),
            "http://127.0.0.1:18884/wallet/musk"
        );

        let mut custom_config = NodeConfig::default();
        custom_config.rpc.wallet = "samplicity".to_string();
        assert_eq!(
            custom_config.rpc.wallet_url(),
            "http://127.0.0.1:18884/wallet/samplicity"
        );
    }

    #[test]
    fn test_wallet_url_trailing_slash() {
        let mut config = NodeConfig::default();
        config.rpc.url = "http://127.0.0.1:18884/".to_string();
        // Should handle trailing slash gracefully
        assert_eq!(
            config.rpc.wallet_url(),
            "http://127.0.0.1:18884/wallet/musk"
        );
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
        // Wallet defaults to "musk" when not specified
        assert_eq!(config.rpc.wallet, "musk");
    }

    #[test]
    fn test_parse_toml_with_wallet() {
        let toml_str = r#"
[network]
network = "testnet"

[rpc]
url = "http://localhost:18891"
wallet = "samplicity"
user = "elements"
password = "elementspass"
"#;
        let config = NodeConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.rpc.wallet, "samplicity");
        assert_eq!(
            config.rpc.wallet_url(),
            "http://localhost:18891/wallet/samplicity"
        );
    }

    #[test]
    fn test_parse_toml_liquid_network() {
        let toml_str = r#"
[network]
network = "liquidv1"

[rpc]
url = "http://localhost:7041"
user = "user"
password = "pass"
"#;
        let config = NodeConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.network(), Network::Liquid);
    }

    #[test]
    fn test_parse_toml_invalid() {
        let toml_str = "this is not valid toml {{{";
        let result = NodeConfig::from_toml(toml_str);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Parse(_)));
    }

    #[test]
    fn test_network_params() {
        assert_eq!(Network::Regtest.default_rpc_port(), 18884);
        assert_eq!(Network::Testnet.default_rpc_port(), 18892);
        assert_eq!(Network::Liquid.default_rpc_port(), 7041);
    }

    #[test]
    fn test_network_address_params() {
        // Verify we get the correct address params for each network
        let regtest_params = Network::Regtest.address_params();
        let testnet_params = Network::Testnet.address_params();
        let liquid_params = Network::Liquid.address_params();

        // All should return valid params (different from each other)
        assert_ne!(regtest_params.bech_hrp, testnet_params.bech_hrp);
        assert_ne!(testnet_params.bech_hrp, liquid_params.bech_hrp);
    }

    #[test]
    fn test_network_default_rpc_url() {
        assert_eq!(Network::Regtest.default_rpc_url(), "http://127.0.0.1:18884");
        assert_eq!(Network::Testnet.default_rpc_url(), "http://127.0.0.1:18892");
        assert_eq!(Network::Liquid.default_rpc_url(), "http://127.0.0.1:7041");
    }

    #[test]
    fn test_network_display() {
        assert_eq!(format!("{}", Network::Regtest), "regtest");
        assert_eq!(format!("{}", Network::Testnet), "testnet");
        assert_eq!(format!("{}", Network::Liquid), "liquidv1");
    }

    #[test]
    fn test_network_default() {
        let network: Network = Default::default();
        assert_eq!(network, Network::Regtest);
    }

    #[test]
    fn test_rpc_config_default() {
        let rpc = RpcConfig::default();
        assert_eq!(rpc.url, "http://127.0.0.1:18884");
        assert_eq!(rpc.user, "user");
        assert_eq!(rpc.password, "password");
        assert_eq!(rpc.wallet, "musk");
    }

    #[test]
    fn test_rpc_config_for_network() {
        let regtest_rpc = RpcConfig::for_network(Network::Regtest);
        assert_eq!(regtest_rpc.url, "http://127.0.0.1:18884");

        let testnet_rpc = RpcConfig::for_network(Network::Testnet);
        assert_eq!(testnet_rpc.url, "http://127.0.0.1:18892");

        let liquid_rpc = RpcConfig::for_network(Network::Liquid);
        assert_eq!(liquid_rpc.url, "http://127.0.0.1:7041");
    }

    #[test]
    fn test_node_config_testnet() {
        let config = NodeConfig::testnet();
        assert_eq!(config.network(), Network::Testnet);
        assert_eq!(config.rpc.url, "http://127.0.0.1:18892");
    }

    #[test]
    fn test_node_config_liquid() {
        let config = NodeConfig::liquid();
        assert_eq!(config.network(), Network::Liquid);
        assert_eq!(config.rpc.url, "http://127.0.0.1:7041");
    }

    #[test]
    fn test_node_config_set_network() {
        let mut config = NodeConfig::regtest();
        assert_eq!(config.network(), Network::Regtest);

        config.set_network(Network::Testnet);
        assert_eq!(config.network(), Network::Testnet);

        config.set_network(Network::Liquid);
        assert_eq!(config.network(), Network::Liquid);
    }

    #[test]
    fn test_node_config_with_rpc() {
        let config = NodeConfig::regtest().with_rpc("http://custom:1234", "myuser", "mypass");

        assert_eq!(config.rpc.url, "http://custom:1234");
        assert_eq!(config.rpc.user, "myuser");
        assert_eq!(config.rpc.password, "mypass");
        // Wallet should be preserved
        assert_eq!(config.rpc.wallet, "musk");
    }

    #[test]
    fn test_node_config_with_wallet() {
        let config = NodeConfig::regtest().with_wallet("custom_wallet");

        assert_eq!(config.rpc.wallet, "custom_wallet");
    }

    #[test]
    fn test_node_config_with_genesis_hash() {
        let config = NodeConfig::regtest()
            .with_genesis_hash("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206");

        assert_eq!(
            config.chain.genesis_hash,
            Some("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206".to_string())
        );
    }

    #[test]
    fn test_node_config_genesis_hash_missing() {
        let config = NodeConfig::regtest();
        let result = config.genesis_hash();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MissingGenesisHash
        ));
    }

    #[test]
    fn test_node_config_genesis_hash_invalid() {
        let config = NodeConfig::regtest().with_genesis_hash("not_a_valid_hash");
        let result = config.genesis_hash();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidGenesisHash(_)
        ));
    }

    #[test]
    fn test_node_config_genesis_hash_valid() {
        let config = NodeConfig::regtest()
            .with_genesis_hash("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206");
        let hash = config.genesis_hash().unwrap();
        assert_eq!(
            hash.to_string(),
            "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
        );
    }

    #[test]
    fn test_node_config_address_params() {
        let regtest = NodeConfig::regtest();
        let testnet = NodeConfig::testnet();
        let liquid = NodeConfig::liquid();

        // Verify each config returns different address params
        assert_ne!(
            regtest.address_params().bech_hrp,
            testnet.address_params().bech_hrp
        );
        assert_ne!(
            testnet.address_params().bech_hrp,
            liquid.address_params().bech_hrp
        );
    }

    #[test]
    fn test_node_config_to_toml() {
        let config = NodeConfig::regtest()
            .with_rpc("http://localhost:18884", "testuser", "testpass")
            .with_wallet("test_wallet")
            .with_genesis_hash("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206");

        let toml_str = config.to_toml().unwrap();

        // Parse it back
        let parsed = NodeConfig::from_toml(&toml_str).unwrap();
        assert_eq!(parsed.network(), Network::Regtest);
        assert_eq!(parsed.rpc.url, "http://localhost:18884");
        assert_eq!(parsed.rpc.user, "testuser");
        assert_eq!(parsed.rpc.password, "testpass");
        assert_eq!(parsed.rpc.wallet, "test_wallet");
        assert_eq!(
            parsed.chain.genesis_hash,
            Some("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206".to_string())
        );
    }

    #[test]
    fn test_node_config_from_file() {
        let toml_content = r#"
[network]
network = "testnet"

[rpc]
url = "http://localhost:18892"
user = "fileuser"
password = "filepass"
wallet = "file_wallet"

[chain]
genesis_hash = "abc123"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();

        let config = NodeConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(config.network(), Network::Testnet);
        assert_eq!(config.rpc.user, "fileuser");
        assert_eq!(config.rpc.wallet, "file_wallet");
    }

    #[test]
    fn test_node_config_from_file_not_found() {
        let result = NodeConfig::from_file("/nonexistent/path/config.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Io(_)));
    }

    #[test]
    fn test_node_config_save() {
        let config =
            NodeConfig::testnet().with_rpc("http://localhost:18892", "saveuser", "savepass");

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path()).unwrap();

        // Read back
        let loaded = NodeConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(loaded.network(), Network::Testnet);
        assert_eq!(loaded.rpc.user, "saveuser");
    }

    #[test]
    fn test_config_error_display() {
        let io_err = ConfigError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
        assert!(io_err.to_string().contains("IO error"));

        let missing = ConfigError::MissingGenesisHash;
        assert!(missing.to_string().contains("genesis hash"));

        let invalid = ConfigError::InvalidGenesisHash("bad hash".to_string());
        assert!(invalid.to_string().contains("Invalid genesis hash"));
    }

    #[test]
    fn test_chain_config_default() {
        let chain = ChainConfig::default();
        assert!(chain.genesis_hash.is_none());
    }

    #[test]
    fn test_builder_chain() {
        // Test chaining multiple builder methods
        let config = NodeConfig::regtest()
            .with_rpc("http://custom:1234", "u", "p")
            .with_wallet("w")
            .with_genesis_hash("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206");

        assert_eq!(config.rpc.url, "http://custom:1234");
        assert_eq!(config.rpc.user, "u");
        assert_eq!(config.rpc.password, "p");
        assert_eq!(config.rpc.wallet, "w");
        assert!(config.chain.genesis_hash.is_some());
    }
}
