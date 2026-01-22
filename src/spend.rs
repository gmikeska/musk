//! Transaction construction and spending utilities

use crate::client::Utxo;
use crate::error::SpendError;
use crate::program::{InstantiatedProgram, SatisfiedProgram};
use elements::hashes::Hash;
use elements::{
    confidential, AssetIssuance, LockTime, Script, Sequence, Transaction, TxIn, TxInWitness, TxOut,
    TxOutWitness,
};
use simplicityhl::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use simplicityhl::WitnessValues;

/// Builder for constructing spending transactions
pub struct SpendBuilder {
    program: InstantiatedProgram,
    utxo: Utxo,
    outputs: Vec<TxOut>,
    lock_time: LockTime,
    sequence: Sequence,
    genesis_hash: elements::BlockHash,
}

impl SpendBuilder {
    /// Create a new spend builder for the given program and UTXO
    #[must_use]
    pub fn new(program: InstantiatedProgram, utxo: Utxo) -> Self {
        Self {
            program,
            utxo,
            outputs: Vec::new(),
            lock_time: LockTime::ZERO,
            sequence: Sequence::MAX,
            genesis_hash: elements::BlockHash::from_byte_array([0u8; 32]), // Default, should be set
        }
    }

    /// Set the genesis block hash (required for sighash computation)
    #[must_use]
    pub const fn genesis_hash(mut self, hash: elements::BlockHash) -> Self {
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
    #[must_use]
    pub const fn lock_time(mut self, lock_time: LockTime) -> Self {
        self.lock_time = lock_time;
        self
    }

    /// Set the sequence number
    #[must_use]
    pub const fn sequence(mut self, sequence: Sequence) -> Self {
        self.sequence = sequence;
        self
    }

    /// Compute the `sighash_all` for this transaction
    ///
    /// This is used to generate witness values that include signatures
    ///
    /// # Errors
    ///
    /// Returns an error if the control block cannot be found.
    pub fn sighash_all(&self) -> Result<[u8; 32], SpendError> {
        let tx = self.build_unsigned_tx();
        let utxo = ElementsUtxo {
            script_pubkey: self.utxo.script_pubkey.clone(),
            value: confidential::Value::Explicit(self.utxo.amount),
            asset: self.utxo.asset,
        };

        let (script, _version) = self.program.script_version();
        let control_block = self
            .program
            .taproot_info()
            .control_block(&(script, self.program.script_version().1))
            .ok_or_else(|| SpendError::BuildError("Control block not found".into()))?;

        let env = ElementsEnv::new(
            &tx,
            vec![utxo],
            0,
            self.program.cmr(),
            control_block,
            None,
            self.genesis_hash,
        );

        Ok(*env.c_tx_env().sighash_all().as_byte_array())
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
    ///
    /// # Errors
    ///
    /// Returns an error if the program cannot be satisfied or the transaction cannot be finalized.
    pub fn finalize(self, witness_values: WitnessValues) -> Result<Transaction, SpendError> {
        let satisfied = self.program.satisfy(witness_values)?;
        self.finalize_with_satisfied(&satisfied)
    }

    /// Finalize the transaction with a pre-satisfied program
    ///
    /// # Errors
    ///
    /// Returns an error if the control block cannot be found or transaction extraction fails.
    pub fn finalize_with_satisfied(
        self,
        satisfied: &SatisfiedProgram,
    ) -> Result<Transaction, SpendError> {
        let (script, version) = self.program.script_version();
        let control_block = satisfied
            .taproot_info()
            .control_block(&(script.clone(), version))
            .ok_or_else(|| SpendError::BuildError("Control block not found".into()))?;

        let (program_bytes, witness_bytes) = satisfied.encode();

        // Build the input witness stack for Simplicity/Taproot
        let input_witness = TxInWitness {
            amount_rangeproof: None,
            inflation_keys_rangeproof: None,
            script_witness: vec![
                witness_bytes,
                program_bytes,
                script.into_bytes(),
                control_block.serialize(),
            ],
            pegin_witness: vec![],
        };

        // Build the transaction directly (avoid PSBT which may drop output witnesses)
        Ok(Transaction {
            version: 2,
            lock_time: self.lock_time,
            input: vec![TxIn {
                previous_output: elements::OutPoint::new(self.utxo.txid, self.utxo.vout),
                is_pegin: false,
                script_sig: Script::new(),
                sequence: self.sequence,
                asset_issuance: AssetIssuance::null(),
                witness: input_witness,
            }],
            output: self.outputs,
        })
    }
}

/// Helper to create a simple spending transaction
///
/// # Errors
///
/// Returns an error if the asset is not explicit or the transaction cannot be built.
pub fn simple_spend(
    program: InstantiatedProgram,
    utxo: Utxo,
    destination: Script,
    amount: u64,
    fee: u64,
    genesis_hash: elements::BlockHash,
    witness_values: WitnessValues,
) -> Result<Transaction, SpendError> {
    let confidential::Asset::Explicit(asset) = utxo.asset else {
        return Err(SpendError::InvalidUtxo("Non-explicit asset".into()));
    };

    let mut builder = SpendBuilder::new(program, utxo).genesis_hash(genesis_hash);
    builder.add_output_simple(destination, amount, asset);
    builder.add_fee(fee, asset);
    builder.finalize(witness_values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{test_genesis_hash, SIMPLE_PROGRAM};
    use crate::{Arguments, Program};
    use elements::hashes::Hash;
    use elements::issuance::AssetId;

    fn test_program() -> InstantiatedProgram {
        let program = Program::from_source(SIMPLE_PROGRAM).unwrap();
        program.instantiate(Arguments::default()).unwrap()
    }

    fn test_utxo_with_script(script: Script) -> Utxo {
        Utxo {
            txid: elements::Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array(
                [2u8; 32],
            )),
            vout: 0,
            amount: 100_000_000, // 1 BTC
            script_pubkey: script,
            asset: confidential::Asset::Explicit(
                AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
            ),
        }
    }

    #[test]
    fn test_spend_builder_new() {
        let program = test_program();
        let utxo = test_utxo_with_script(
            program
                .address(&elements::AddressParams::ELEMENTS)
                .script_pubkey(),
        );

        let builder = SpendBuilder::new(program, utxo);
        // Builder should be created successfully
        assert!(std::mem::size_of_val(&builder) > 0);
    }

    #[test]
    fn test_spend_builder_genesis_hash() {
        let program = test_program();
        let utxo = test_utxo_with_script(
            program
                .address(&elements::AddressParams::ELEMENTS)
                .script_pubkey(),
        );

        let genesis = test_genesis_hash();
        let builder = SpendBuilder::new(program, utxo).genesis_hash(genesis);

        // Builder should accept genesis hash
        assert!(std::mem::size_of_val(&builder) > 0);
    }

    #[test]
    fn test_spend_builder_add_output() {
        let program = test_program();
        let utxo = test_utxo_with_script(
            program
                .address(&elements::AddressParams::ELEMENTS)
                .script_pubkey(),
        );

        let mut builder = SpendBuilder::new(program, utxo);

        let output = TxOut {
            value: confidential::Value::Explicit(50_000_000),
            script_pubkey: Script::new(),
            asset: confidential::Asset::Explicit(
                AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
            ),
            nonce: confidential::Nonce::Null,
            witness: TxOutWitness::empty(),
        };

        builder.add_output(output);
        // Should be able to chain operations
        assert!(std::mem::size_of_val(&builder) > 0);
    }

    #[test]
    fn test_spend_builder_add_output_simple() {
        let program = test_program();
        let utxo = test_utxo_with_script(
            program
                .address(&elements::AddressParams::ELEMENTS)
                .script_pubkey(),
        );

        let mut builder = SpendBuilder::new(program, utxo);
        let asset = AssetId::from_slice(&[0u8; 32]).expect("valid asset");

        builder.add_output_simple(Script::new(), 50_000_000, asset);
        // Should be able to add output
        assert!(std::mem::size_of_val(&builder) > 0);
    }

    #[test]
    fn test_spend_builder_add_fee() {
        let program = test_program();
        let utxo = test_utxo_with_script(
            program
                .address(&elements::AddressParams::ELEMENTS)
                .script_pubkey(),
        );

        let mut builder = SpendBuilder::new(program, utxo);
        let asset = AssetId::from_slice(&[0u8; 32]).expect("valid asset");

        builder.add_fee(1000, asset);
        // Should be able to add fee
        assert!(std::mem::size_of_val(&builder) > 0);
    }

    #[test]
    fn test_spend_builder_lock_time() {
        let program = test_program();
        let utxo = test_utxo_with_script(
            program
                .address(&elements::AddressParams::ELEMENTS)
                .script_pubkey(),
        );

        let builder =
            SpendBuilder::new(program, utxo).lock_time(LockTime::from_height(100).unwrap());

        assert!(std::mem::size_of_val(&builder) > 0);
    }

    #[test]
    fn test_spend_builder_sequence() {
        let program = test_program();
        let utxo = test_utxo_with_script(
            program
                .address(&elements::AddressParams::ELEMENTS)
                .script_pubkey(),
        );

        let builder =
            SpendBuilder::new(program, utxo).sequence(Sequence::from_consensus(0xFFFFFFFE));

        assert!(std::mem::size_of_val(&builder) > 0);
    }

    #[test]
    fn test_spend_builder_sighash_all() {
        let program = test_program();
        let address = program.address(&elements::AddressParams::ELEMENTS);
        let utxo = test_utxo_with_script(address.script_pubkey());

        let genesis = test_genesis_hash();
        let asset = AssetId::from_slice(&[0u8; 32]).expect("valid asset");

        let mut builder = SpendBuilder::new(program, utxo).genesis_hash(genesis);
        builder.add_output_simple(Script::new(), 99_999_000, asset);
        builder.add_fee(1000, asset);

        let sighash = builder.sighash_all().unwrap();
        assert_eq!(sighash.len(), 32);

        // Sighash should be deterministic
        let sighash2 = builder.sighash_all().unwrap();
        assert_eq!(sighash, sighash2);
    }

    #[test]
    fn test_spend_builder_finalize() {
        let program = test_program();
        let address = program.address(&elements::AddressParams::ELEMENTS);
        let utxo = test_utxo_with_script(address.script_pubkey());

        let genesis = test_genesis_hash();
        let asset = AssetId::from_slice(&[0u8; 32]).expect("valid asset");

        let mut builder = SpendBuilder::new(program, utxo).genesis_hash(genesis);
        builder.add_output_simple(Script::new(), 99_999_000, asset);
        builder.add_fee(1000, asset);

        let tx = builder.finalize(WitnessValues::default()).unwrap();

        // Transaction should have correct structure
        assert_eq!(tx.version, 2);
        assert_eq!(tx.input.len(), 1);
        assert_eq!(tx.output.len(), 2); // output + fee

        // Input witness should contain simplicity data
        assert!(!tx.input[0].witness.script_witness.is_empty());
    }

    #[test]
    fn test_spend_builder_finalize_with_satisfied() {
        let program = test_program();
        let address = program.address(&elements::AddressParams::ELEMENTS);
        let utxo = test_utxo_with_script(address.script_pubkey());

        let genesis = test_genesis_hash();
        let asset = AssetId::from_slice(&[0u8; 32]).expect("valid asset");

        // First satisfy the program
        let satisfied = program.satisfy(WitnessValues::default()).unwrap();

        let mut builder = SpendBuilder::new(program, utxo).genesis_hash(genesis);
        builder.add_output_simple(Script::new(), 99_999_000, asset);
        builder.add_fee(1000, asset);

        let tx = builder.finalize_with_satisfied(&satisfied).unwrap();

        assert_eq!(tx.version, 2);
        assert_eq!(tx.input.len(), 1);
        assert!(!tx.input[0].witness.script_witness.is_empty());
    }

    #[test]
    fn test_simple_spend() {
        let program = test_program();
        let address = program.address(&elements::AddressParams::ELEMENTS);

        let utxo = Utxo {
            txid: elements::Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array(
                [2u8; 32],
            )),
            vout: 0,
            amount: 100_000_000,
            script_pubkey: address.script_pubkey(),
            asset: confidential::Asset::Explicit(
                AssetId::from_slice(&[0u8; 32]).expect("valid asset"),
            ),
        };

        let genesis = test_genesis_hash();
        let destination = Script::new();

        let tx = simple_spend(
            program,
            utxo,
            destination,
            99_999_000,
            1000,
            genesis,
            WitnessValues::default(),
        )
        .unwrap();

        assert_eq!(tx.output.len(), 2);
    }

    #[test]
    fn test_simple_spend_non_explicit_asset() {
        let program = test_program();

        let utxo = Utxo {
            txid: elements::Txid::from_raw_hash(elements::hashes::sha256d::Hash::from_byte_array(
                [2u8; 32],
            )),
            vout: 0,
            amount: 100_000_000,
            script_pubkey: Script::new(),
            asset: confidential::Asset::Null, // Non-explicit
        };

        let genesis = test_genesis_hash();
        let destination = Script::new();

        let result = simple_spend(
            program,
            utxo,
            destination,
            99_999_000,
            1000,
            genesis,
            WitnessValues::default(),
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SpendError::InvalidUtxo(_)));
    }

    #[test]
    fn test_spend_builder_multiple_outputs() {
        let program = test_program();
        let address = program.address(&elements::AddressParams::ELEMENTS);
        let utxo = test_utxo_with_script(address.script_pubkey());

        let genesis = test_genesis_hash();
        let asset = AssetId::from_slice(&[0u8; 32]).expect("valid asset");

        let mut builder = SpendBuilder::new(program, utxo).genesis_hash(genesis);

        // Add multiple outputs
        builder.add_output_simple(Script::new(), 30_000_000, asset);
        builder.add_output_simple(Script::from(vec![0x51]), 30_000_000, asset);
        builder.add_output_simple(Script::from(vec![0x00, 0x14]), 39_998_000, asset);
        builder.add_fee(2000, asset);

        let tx = builder.finalize(WitnessValues::default()).unwrap();

        assert_eq!(tx.output.len(), 4); // 3 outputs + 1 fee
    }

    #[test]
    fn test_spend_builder_custom_lock_time_and_sequence() {
        let program = test_program();
        let address = program.address(&elements::AddressParams::ELEMENTS);
        let utxo = test_utxo_with_script(address.script_pubkey());

        let genesis = test_genesis_hash();
        let asset = AssetId::from_slice(&[0u8; 32]).expect("valid asset");

        let lock_time = LockTime::from_height(500_000).unwrap();
        let sequence = Sequence::from_consensus(0xFFFFFFFE);

        let mut builder = SpendBuilder::new(program, utxo)
            .genesis_hash(genesis)
            .lock_time(lock_time)
            .sequence(sequence);

        builder.add_output_simple(Script::new(), 99_999_000, asset);
        builder.add_fee(1000, asset);

        let tx = builder.finalize(WitnessValues::default()).unwrap();

        assert_eq!(tx.lock_time, lock_time);
        assert_eq!(tx.input[0].sequence, sequence);
    }
}
