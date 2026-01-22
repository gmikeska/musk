//! Abstract interface for interacting with Elements nodes

use crate::error::ProgramError;
use elements::hashes::Hash;
use elements::{Address, BlockHash, Transaction, Txid};

/// Result type for node client operations
pub type ClientResult<T> = Result<T, ProgramError>;

/// UTXO representation for spending
///
/// For confidential UTXOs, the blinding fields contain the data needed to
/// spend the UTXO. These are obtained from `listunspent` when the blinding
/// key is imported to the wallet.
#[derive(Debug, Clone)]
pub struct Utxo {
    pub txid: Txid,
    pub vout: u32,
    /// The unblinded amount in satoshis
    pub amount: u64,
    pub script_pubkey: elements::Script,
    /// The asset (explicit asset ID, used for transaction construction)
    pub asset: elements::confidential::Asset,
    /// Amount blinding factor (32 bytes, from listunspent "amountblinder")
    /// None for explicit UTXOs, Some for confidential UTXOs
    pub amount_blinder: Option<[u8; 32]>,
    /// Asset blinding factor (32 bytes, from listunspent "assetblinder")
    /// None for explicit UTXOs, Some for confidential UTXOs
    pub asset_blinder: Option<[u8; 32]>,
    /// Amount commitment (33 bytes, from listunspent "amountcommitment")
    /// None for explicit UTXOs, Some for confidential UTXOs
    pub amount_commitment: Option<[u8; 33]>,
    /// Asset commitment (33 bytes, from listunspent "assetcommitment")
    /// None for explicit UTXOs, Some for confidential UTXOs
    pub asset_commitment: Option<[u8; 33]>,
}

impl Utxo {
    /// Check if this UTXO is from a confidential transaction (has non-zero blinders)
    #[must_use]
    pub fn is_confidential(&self) -> bool {
        // A UTXO is confidential if it has non-zero blinders
        if let Some(amount_blinder) = &self.amount_blinder {
            if amount_blinder.iter().any(|&b| b != 0) {
                return true;
            }
        }
        false
    }

    /// Get the amount blinder as a hex string (for RPC calls)
    #[must_use]
    pub fn amount_blinder_hex(&self) -> String {
        use elements::hex::ToHex;
        self.amount_blinder
            .map(|b| b.to_hex())
            .unwrap_or_else(|| "0".repeat(64))
    }

    /// Get the asset blinder as a hex string (for RPC calls)
    #[must_use]
    pub fn asset_blinder_hex(&self) -> String {
        use elements::hex::ToHex;
        self.asset_blinder
            .map(|b| b.to_hex())
            .unwrap_or_else(|| "0".repeat(64))
    }
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
            // Blinding data must be set by caller if available
            amount_blinder: None,
            asset_blinder: None,
            amount_commitment: None,
            asset_commitment: None,
        }
    }
}

impl Default for Utxo {
    fn default() -> Self {
        Self {
            txid: Txid::from_byte_array([0u8; 32]),
            vout: 0,
            amount: 0,
            script_pubkey: elements::Script::new(),
            asset: elements::confidential::Asset::Null,
            amount_blinder: None,
            asset_blinder: None,
            amount_commitment: None,
            asset_commitment: None,
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
            amount_blinder: None,
            asset_blinder: None,
            amount_commitment: None,
            asset_commitment: None,
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
            amount_blinder: None,
            asset_blinder: None,
            amount_commitment: None,
            asset_commitment: None,
        };

        let cloned = utxo.clone();
        assert_eq!(utxo.txid, cloned.txid);
        assert_eq!(utxo.vout, cloned.vout);
        assert_eq!(utxo.amount, cloned.amount);
        assert_eq!(utxo.script_pubkey, cloned.script_pubkey);
    }

    #[test]
    fn test_utxo_is_confidential() {
        use elements::hashes::Hash;

        // Explicit UTXO (no blinders)
        let explicit_utxo = Utxo {
            txid: Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array([1u8; 32])),
            vout: 0,
            amount: 100_000,
            script_pubkey: elements::Script::new(),
            asset: confidential::Asset::Explicit(
                AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
            ),
            amount_blinder: None,
            asset_blinder: None,
            amount_commitment: None,
            asset_commitment: None,
        };
        assert!(!explicit_utxo.is_confidential());

        // Explicit UTXO with zero blinders (still explicit)
        let zero_blinder_utxo = Utxo {
            amount_blinder: Some([0u8; 32]),
            asset_blinder: Some([0u8; 32]),
            ..explicit_utxo.clone()
        };
        assert!(!zero_blinder_utxo.is_confidential());

        // Confidential UTXO (has non-zero blinders)
        let confidential_utxo = Utxo {
            amount_blinder: Some([1u8; 32]),
            asset_blinder: Some([2u8; 32]),
            amount_commitment: Some([3u8; 33]),
            asset_commitment: Some([4u8; 33]),
            ..explicit_utxo
        };
        assert!(confidential_utxo.is_confidential());
    }

    #[test]
    fn test_utxo_blinder_hex() {
        let utxo_no_blinder = Utxo::default();
        assert_eq!(utxo_no_blinder.amount_blinder_hex(), "0".repeat(64));
        assert_eq!(utxo_no_blinder.asset_blinder_hex(), "0".repeat(64));

        let utxo_with_blinder = Utxo {
            amount_blinder: Some([0xab; 32]),
            asset_blinder: Some([0xcd; 32]),
            ..Utxo::default()
        };
        assert_eq!(utxo_with_blinder.amount_blinder_hex(), "ab".repeat(32));
        assert_eq!(utxo_with_blinder.asset_blinder_hex(), "cd".repeat(32));
    }
}
