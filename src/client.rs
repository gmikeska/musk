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

#[cfg(test)]
mod tests {
    use super::*;
    use elements::confidential;
    use elements::issuance::AssetId;

    #[test]
    fn test_utxo_from_txout_explicit_value() {
        let txout = elements::TxOut {
            value: confidential::Value::Explicit(100_000_000),
            script_pubkey: elements::Script::from(vec![0x00, 0x14, 0xab]),
            asset: confidential::Asset::Explicit(
                AssetId::from_slice(&[42u8; 32]).expect("valid asset"),
            ),
            nonce: confidential::Nonce::Null,
            witness: elements::TxOutWitness::empty(),
        };

        let utxo: Utxo = txout.into();

        // txid and vout are set to defaults - caller must update them
        assert_eq!(utxo.txid, Txid::from_byte_array([0u8; 32]));
        assert_eq!(utxo.vout, 0);
        assert_eq!(utxo.amount, 100_000_000);
        assert_eq!(
            utxo.script_pubkey,
            elements::Script::from(vec![0x00, 0x14, 0xab])
        );
    }

    #[test]
    fn test_utxo_from_txout_confidential_value() {
        // Confidential values return 0 for amount
        let txout = elements::TxOut {
            value: confidential::Value::Null,
            script_pubkey: elements::Script::new(),
            asset: confidential::Asset::Null,
            nonce: confidential::Nonce::Null,
            witness: elements::TxOutWitness::empty(),
        };

        let utxo: Utxo = txout.into();

        // Non-explicit values result in amount = 0
        assert_eq!(utxo.amount, 0);
    }

    #[test]
    fn test_utxo_debug() {
        use elements::hashes::Hash;

        let utxo = Utxo {
            txid: Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array([1u8; 32])),
            vout: 1,
            amount: 50_000_000,
            script_pubkey: elements::Script::new(),
            asset: confidential::Asset::Null,
        };

        let debug_str = format!("{:?}", utxo);
        assert!(debug_str.contains("Utxo"));
        assert!(debug_str.contains("vout: 1"));
        assert!(debug_str.contains("amount: 50000000"));
    }

    #[test]
    fn test_utxo_clone() {
        use elements::hashes::Hash;

        let utxo = Utxo {
            txid: Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array([5u8; 32])),
            vout: 2,
            amount: 25_000_000,
            script_pubkey: elements::Script::from(vec![0x51]),
            asset: confidential::Asset::Explicit(
                AssetId::from_slice(&[10u8; 32]).expect("valid asset"),
            ),
        };

        let cloned = utxo.clone();
        assert_eq!(utxo.txid, cloned.txid);
        assert_eq!(utxo.vout, cloned.vout);
        assert_eq!(utxo.amount, cloned.amount);
        assert_eq!(utxo.script_pubkey, cloned.script_pubkey);
    }
}
