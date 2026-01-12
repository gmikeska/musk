//! Transaction construction and spending utilities

use crate::client::Utxo;
use crate::contract::{CompiledContract, SatisfiedContract};
use crate::error::SpendError;
use elements::pset::PartiallySignedTransaction as Psbt;
use elements::{confidential, AssetIssuance, LockTime, Script, Sequence, Transaction, TxIn, TxInWitness, TxOut, TxOutWitness};
use simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use simplicityhl::WitnessValues;
use std::sync::Arc;

/// Builder for constructing spending transactions
pub struct SpendBuilder {
    contract: CompiledContract,
    utxo: Utxo,
    outputs: Vec<TxOut>,
    lock_time: LockTime,
    sequence: Sequence,
    genesis_hash: elements::BlockHash,
}

impl SpendBuilder {
    /// Create a new spend builder for the given contract and UTXO
    pub fn new(contract: CompiledContract, utxo: Utxo) -> Self {
        Self {
            contract,
            utxo,
            outputs: Vec::new(),
            lock_time: LockTime::ZERO,
            sequence: Sequence::MAX,
            genesis_hash: elements::BlockHash::all_zeros(), // Default, should be set
        }
    }

    /// Set the genesis block hash (required for sighash computation)
    pub fn genesis_hash(mut self, hash: elements::BlockHash) -> Self {
        self.genesis_hash = hash;
        self
    }

    /// Add an output to the transaction
    pub fn add_output(&mut self, output: TxOut) -> &mut Self {
        self.outputs.push(output);
        self
    }

    /// Add a simple output with explicit value
    pub fn add_output_simple(
        &mut self,
        script_pubkey: Script,
        amount: u64,
        asset: elements::AssetId,
    ) -> &mut Self {
        self.outputs.push(TxOut {
            value: confidential::Value::Explicit(amount),
            script_pubkey,
            asset: confidential::Asset::Explicit(asset),
            nonce: confidential::Nonce::Null,
            witness: TxOutWitness::empty(),
        });
        self
    }

    /// Add a fee output
    pub fn add_fee(&mut self, amount: u64, asset: elements::AssetId) -> &mut Self {
        self.outputs.push(TxOut::new_fee(amount, asset));
        self
    }

    /// Set the lock time
    pub fn lock_time(mut self, lock_time: LockTime) -> Self {
        self.lock_time = lock_time;
        self
    }

    /// Set the sequence number
    pub fn sequence(mut self, sequence: Sequence) -> Self {
        self.sequence = sequence;
        self
    }

    /// Compute the sighash_all for this transaction
    ///
    /// This is used to generate witness values that include signatures
    pub fn sighash_all(&self) -> Result<[u8; 32], SpendError> {
        let tx = self.build_unsigned_tx();
        let utxo = ElementsUtxo {
            script_pubkey: self.utxo.script_pubkey.clone(),
            value: self.utxo.amount,
            asset: match self.utxo.asset {
                confidential::Asset::Explicit(id) => id,
                _ => return Err(SpendError::InvalidUtxo("Non-explicit asset".into())),
            },
        };

        let (script, _version) = self.contract.script_version();
        let control_block = self
            .contract
            .taproot_info()
            .control_block(&(script, self.contract.script_version().1))
            .ok_or_else(|| SpendError::BuildError("Control block not found".into()))?;

        let env = ElementsEnv::new(
            &tx,
            vec![utxo],
            0,
            self.contract.cmr(),
            control_block,
            None,
            self.genesis_hash,
        );

        Ok(env.c_tx_env().sighash_all().to_byte_array())
    }

    /// Build the unsigned transaction
    fn build_unsigned_tx(&self) -> Transaction {
        Transaction {
            version: 2,
            lock_time: self.lock_time,
            input: vec![TxIn {
                previous_output: elements::OutPoint::new(self.utxo.txid, self.utxo.vout),
                is_pegin: false,
                script_sig: Script::new(),
                sequence: self.sequence,
                asset_issuance: AssetIssuance::null(),
                witness: TxInWitness::empty(),
            }],
            output: self.outputs.clone(),
        }
    }

    /// Finalize the transaction with witness values
    pub fn finalize(self, witness_values: WitnessValues) -> Result<Transaction, SpendError> {
        let satisfied = self.contract.satisfy(witness_values)?;
        self.finalize_with_satisfied(satisfied)
    }

    /// Finalize the transaction with a pre-satisfied contract
    pub fn finalize_with_satisfied(self, satisfied: SatisfiedContract) -> Result<Transaction, SpendError> {
        let mut psbt = Psbt::from_tx(self.build_unsigned_tx());

        let (script, version) = self.contract.script_version();
        let control_block = satisfied
            .taproot_info()
            .control_block(&(script.clone(), version))
            .ok_or_else(|| SpendError::BuildError("Control block not found".into()))?;

        let (program_bytes, witness_bytes) = satisfied.encode();

        psbt.inputs_mut()[0].final_script_witness = Some(vec![
            witness_bytes,
            program_bytes,
            script.into_bytes(),
            control_block.serialize(),
        ]);

        psbt.extract_tx()
            .map_err(|e| SpendError::FinalizationError(e.to_string()))
    }
}

/// Helper to create a simple spending transaction
pub fn simple_spend(
    contract: CompiledContract,
    utxo: Utxo,
    destination: Script,
    amount: u64,
    fee: u64,
    genesis_hash: elements::BlockHash,
    witness_values: WitnessValues,
) -> Result<Transaction, SpendError> {
    let asset = match utxo.asset {
        confidential::Asset::Explicit(id) => id,
        _ => return Err(SpendError::InvalidUtxo("Non-explicit asset".into())),
    };

    let mut builder = SpendBuilder::new(contract, utxo).genesis_hash(genesis_hash);
    builder.add_output_simple(destination, amount, asset);
    builder.add_fee(fee, asset);
    builder.finalize(witness_values)
}

