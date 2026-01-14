//! Mock NodeClient implementation for testing

#![cfg(test)]

use crate::client::{ClientResult, NodeClient, Utxo};
use crate::error::ContractError;
use elements::{Address, BlockHash, Transaction, Txid};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock client for testing without a live node
#[derive(Clone)]
pub struct MockClient {
    inner: Arc<Mutex<MockClientInner>>,
}

struct MockClientInner {
    transactions: HashMap<Txid, Transaction>,
    utxos: HashMap<Address, Vec<Utxo>>,
    block_count: u32,
    genesis_hash: BlockHash,
}

impl MockClient {
    /// Create a new mock client
    #[must_use]
    pub fn new() -> Self {
        use elements::hashes::Hash;

        Self {
            inner: Arc::new(Mutex::new(MockClientInner {
                transactions: HashMap::new(),
                utxos: HashMap::new(),
                block_count: 0,
                genesis_hash: BlockHash::from_raw_hash(
                    elements::hashes::sha256d::Hash::from_byte_array([1u8; 32]),
                ),
            })),
        }
    }

    /// Add a pre-existing transaction to the mock
    pub fn add_transaction(&self, txid: Txid, tx: Transaction) {
        let mut inner = self.inner.lock().unwrap();
        inner.transactions.insert(txid, tx);
    }

    /// Add a UTXO for an address
    pub fn add_utxo(&self, address: Address, utxo: Utxo) {
        let mut inner = self.inner.lock().unwrap();
        inner.utxos.entry(address).or_default().push(utxo);
    }

    /// Get the genesis hash
    #[must_use]
    pub fn genesis_hash(&self) -> BlockHash {
        self.inner.lock().unwrap().genesis_hash
    }

    /// Set the genesis hash
    pub fn set_genesis_hash(&self, hash: BlockHash) {
        self.inner.lock().unwrap().genesis_hash = hash;
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeClient for MockClient {
    fn send_to_address(&self, addr: &Address, amount: u64) -> ClientResult<Txid> {
        use elements::hashes::Hash;
        use elements::issuance::AssetId;
        use elements::{confidential, Script, TxIn, TxInWitness, TxOut, TxOutWitness};

        // Create a mock transaction
        let txid = Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array(
            rand::random::<[u8; 32]>(),
        ));

        let tx = Transaction {
            version: 2,
            lock_time: elements::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: elements::OutPoint::null(),
                is_pegin: false,
                script_sig: Script::new(),
                sequence: elements::Sequence::MAX,
                asset_issuance: elements::AssetIssuance::null(),
                witness: TxInWitness::empty(),
            }],
            output: vec![TxOut {
                value: confidential::Value::Explicit(amount),
                script_pubkey: addr.script_pubkey(),
                asset: confidential::Asset::Explicit(
                    AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
                ),
                nonce: confidential::Nonce::Null,
                witness: TxOutWitness::empty(),
            }],
        };

        // Store the transaction
        let mut inner = self.inner.lock().unwrap();
        inner.transactions.insert(txid, tx.clone());

        // Add UTXO for the address
        inner.utxos.entry(addr.clone()).or_default().push(Utxo {
            txid,
            vout: 0,
            amount,
            script_pubkey: addr.script_pubkey(),
            asset: confidential::Asset::Explicit(
                AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
            ),
        });

        Ok(txid)
    }

    fn get_transaction(&self, txid: &Txid) -> ClientResult<Transaction> {
        let inner = self.inner.lock().unwrap();
        inner
            .transactions
            .get(txid)
            .cloned()
            .ok_or_else(|| ContractError::IoError(std::io::Error::other("Transaction not found")))
    }

    fn broadcast(&self, tx: &Transaction) -> ClientResult<Txid> {
        use elements::hashes::Hash;

        let txid = Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array(
            rand::random::<[u8; 32]>(),
        ));

        let mut inner = self.inner.lock().unwrap();
        inner.transactions.insert(txid, tx.clone());

        Ok(txid)
    }

    fn generate_blocks(&self, count: u32) -> ClientResult<Vec<BlockHash>> {
        use elements::hashes::Hash;

        let mut inner = self.inner.lock().unwrap();
        let mut hashes = Vec::new();

        for _ in 0..count {
            inner.block_count += 1;
            let hash = BlockHash::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array(
                rand::random::<[u8; 32]>(),
            ));
            hashes.push(hash);
        }

        Ok(hashes)
    }

    fn get_utxos(&self, address: &Address) -> ClientResult<Vec<Utxo>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.utxos.get(address).cloned().unwrap_or_default())
    }

    fn get_new_address(&self) -> ClientResult<Address> {
        use elements::bitcoin::PublicKey;
        use elements::AddressParams;
        use secp256k1::Secp256k1;

        let secp = Secp256k1::new();
        let secret_bytes: [u8; 32] = rand::random();
        let secret_key = secp256k1::SecretKey::from_slice(&secret_bytes).map_err(|e| {
            ContractError::IoError(std::io::Error::other(format!("Key error: {e}")))
        })?;
        let secp_pubkey = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let bitcoin_pubkey = PublicKey::new(secp_pubkey);

        Ok(Address::p2wpkh(
            &bitcoin_pubkey,
            None,
            &AddressParams::ELEMENTS,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_send_to_address() {
        let client = MockClient::new();
        let addr = crate::test_fixtures::test_address();

        let txid = client.send_to_address(&addr, 100_000_000).unwrap();

        // Should be able to get the transaction back
        let tx = client.get_transaction(&txid).unwrap();
        assert_eq!(tx.output.len(), 1);
    }

    #[test]
    fn test_mock_get_utxos() {
        let client = MockClient::new();
        let addr = crate::test_fixtures::test_address();

        // Initially no UTXOs
        let utxos = client.get_utxos(&addr).unwrap();
        assert!(utxos.is_empty());

        // Send funds
        client.send_to_address(&addr, 100_000_000).unwrap();

        // Now should have a UTXO
        let utxos = client.get_utxos(&addr).unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].amount, 100_000_000);
    }

    #[test]
    fn test_mock_broadcast() {
        use elements::issuance::AssetId;
        use elements::{confidential, Script, TxIn, TxInWitness, TxOut, TxOutWitness};

        let client = MockClient::new();

        let tx = Transaction {
            version: 2,
            lock_time: elements::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: elements::OutPoint::null(),
                is_pegin: false,
                script_sig: Script::new(),
                sequence: elements::Sequence::MAX,
                asset_issuance: elements::AssetIssuance::null(),
                witness: TxInWitness::empty(),
            }],
            output: vec![TxOut {
                value: confidential::Value::Explicit(50_000_000),
                script_pubkey: Script::new(),
                asset: confidential::Asset::Explicit(
                    AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
                ),
                nonce: confidential::Nonce::Null,
                witness: TxOutWitness::empty(),
            }],
        };

        let txid = client.broadcast(&tx).unwrap();

        // Should be able to retrieve it
        let retrieved_tx = client.get_transaction(&txid).unwrap();
        assert_eq!(retrieved_tx.output.len(), 1);
    }

    #[test]
    fn test_mock_generate_blocks() {
        let client = MockClient::new();

        let hashes = client.generate_blocks(10).unwrap();
        assert_eq!(hashes.len(), 10);
    }

    #[test]
    fn test_mock_get_new_address() {
        let client = MockClient::new();

        let addr1 = client.get_new_address().unwrap();
        let addr2 = client.get_new_address().unwrap();

        // Should generate different addresses
        assert_ne!(addr1.to_string(), addr2.to_string());
    }
}
