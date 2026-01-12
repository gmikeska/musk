//! Contract compilation and instantiation

use crate::address::create_taproot_info;
use crate::error::ContractError;
use elements::taproot::TaprootSpendInfo;
use simplicityhl::{Arguments, CompiledProgram, Parameters, TemplateProgram, WitnessValues};
use std::path::Path;
use std::sync::Arc;

/// A Simplicity contract template with parameterized values
pub struct Contract {
    source: Arc<str>,
    template: TemplateProgram,
}

impl Contract {
    /// Create a contract from source code
    ///
    /// # Errors
    ///
    /// Returns an error if the source code cannot be parsed.
    pub fn from_source(source: &str) -> Result<Self, ContractError> {
        let source = Arc::from(source);
        let template =
            TemplateProgram::new(Arc::clone(&source)).map_err(ContractError::ParseError)?;

        Ok(Self { source, template })
    }

    /// Load a contract from a file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ContractError> {
        let source = std::fs::read_to_string(path)?;
        Self::from_source(&source)
    }

    /// Get the parameters required by this contract template
    #[must_use]
    pub fn parameters(&self) -> &Parameters {
        self.template.parameters()
    }

    /// Instantiate the contract with the given arguments
    ///
    /// # Errors
    ///
    /// Returns an error if instantiation fails or the taproot tree cannot be built.
    pub fn instantiate(&self, arguments: Arguments) -> Result<CompiledContract, ContractError> {
        let compiled = self
            .template
            .instantiate(arguments, false)
            .map_err(ContractError::InstantiationError)?;

        let taproot_info = create_taproot_info(&compiled)?;

        Ok(CompiledContract {
            inner: compiled,
            taproot_info,
        })
    }

    /// Get the source code
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// A compiled Simplicity contract ready for address generation and spending
#[derive(Clone)]
pub struct CompiledContract {
    inner: CompiledProgram,
    taproot_info: TaprootSpendInfo,
}

impl CompiledContract {
    /// Get the commitment Merkle root (CMR) of this contract
    #[must_use]
    pub fn cmr(&self) -> simplicityhl::simplicity::Cmr {
        self.inner.commit().cmr()
    }

    /// Generate a taproot address for this contract
    #[must_use]
    pub fn address(&self, params: &'static elements::AddressParams) -> elements::Address {
        let blinder = None;
        elements::Address::p2tr(
            &secp256k1::Secp256k1::new(),
            self.taproot_info.internal_key(),
            self.taproot_info.merkle_root(),
            blinder,
            params,
        )
    }

    /// Get the taproot spend info
    #[must_use]
    pub const fn taproot_info(&self) -> &TaprootSpendInfo {
        &self.taproot_info
    }

    /// Get the script and leaf version for taproot spending
    #[must_use]
    pub fn script_version(&self) -> (elements::Script, elements::taproot::LeafVersion) {
        let script = elements::script::Script::from(self.cmr().as_ref().to_vec());
        (script, simplicityhl::simplicity::leaf_version())
    }

    /// Satisfy the contract with witness values, producing a satisfied program
    ///
    /// # Errors
    ///
    /// Returns an error if the witness values are invalid or incomplete.
    pub fn satisfy(
        &self,
        witness_values: WitnessValues,
    ) -> Result<SatisfiedContract, ContractError> {
        let satisfied = self
            .inner
            .satisfy(witness_values)
            .map_err(ContractError::SatisfactionError)?;

        Ok(SatisfiedContract {
            inner: satisfied,
            taproot_info: self.taproot_info.clone(),
        })
    }

    /// Get the underlying compiled program
    #[must_use]
    pub const fn inner(&self) -> &CompiledProgram {
        &self.inner
    }
}

/// A satisfied Simplicity contract ready to be encoded in a transaction witness
pub struct SatisfiedContract {
    inner: simplicityhl::SatisfiedProgram,
    taproot_info: TaprootSpendInfo,
}

impl SatisfiedContract {
    /// Get the taproot spend info
    #[must_use]
    pub const fn taproot_info(&self) -> &TaprootSpendInfo {
        &self.taproot_info
    }

    /// Encode the program and witness for inclusion in a transaction
    #[must_use]
    pub fn encode(&self) -> (Vec<u8>, Vec<u8>) {
        self.inner.redeem().to_vec_with_witness()
    }

    /// Get the underlying satisfied program
    #[must_use]
    pub const fn inner(&self) -> &simplicityhl::SatisfiedProgram {
        &self.inner
    }
}
