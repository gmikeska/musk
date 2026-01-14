//! Abstract interface for interacting with Elements nodes

use crate::error::ProgramError;
use elements::hashes::Hash;
use elements::{Address, BlockHash, Transaction, Txid};

/// Result type for node client operations
pub type ClientResult<T> = Result<T, ProgramError>;

/// UTXO representation for spending
#[derive(Debug, Clone)]
pub struct Utxo {
    pub txid: Txid,
    pub vout: u32,
    pub amount: u64,
    pub script_pubkey: elements::Script,
    pub asset: elements::confidential::Asset,
}

impl From<elements::TxOut> for Utxo {
    fn from(txout: elements::TxOut) -> Self {
        Self {
            txid: Txid::from_byte_array([0u8; 32]), // Will be set by caller
            vout: 0,                                // Will be set by caller
            amount: match txout.value {
                elements::confidential::Value::Explicit(amt) => amt,
                _ => 0,
            },
            script_pubkey: txout.script_pubkey,
            asset: txout.asset,
        }
    }
}

/// Abstract interface for interacting with Elements nodes
///
/// This trait allows musk to work with different network backends
/// (regtest, testnet, mainnet) through a unified interface.
pub trait NodeClient {
    /// Send funds to an address
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails or the response is invalid.
    fn send_to_address(&self, addr: &Address, amount: u64) -> ClientResult<Txid>;

    /// Get a transaction by its txid
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction is not found or deserialization fails.
    fn get_transaction(&self, txid: &Txid) -> ClientResult<Transaction>;

    /// Broadcast a transaction to the network
    ///
    /// # Errors
    ///
    /// Returns an error if the broadcast fails or the transaction is rejected.
    fn broadcast(&self, tx: &Transaction) -> ClientResult<Txid>;

    /// Generate blocks (regtest only)
    ///
    /// # Errors
    ///
    /// Returns an error if block generation fails (only works on regtest).
    fn generate_blocks(&self, count: u32) -> ClientResult<Vec<BlockHash>>;

    /// Get UTXOs for an address
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails or the response is invalid.
    fn get_utxos(&self, address: &Address) -> ClientResult<Vec<Utxo>>;

    /// Get a new address from the wallet
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails or the address is invalid.
    fn get_new_address(&self) -> ClientResult<Address>;
}
