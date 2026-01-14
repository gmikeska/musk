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
    /// # Examples
    ///
    /// ```
    /// use musk::Contract;
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// assert!(contract.source().len() > 0);
    /// ```
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
    /// # Examples
    ///
    /// ```ignore
    /// use musk::Contract;
    ///
    /// let contract = Contract::from_file("contract.simf")?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ContractError> {
        let source = std::fs::read_to_string(path)?;
        Self::from_source(&source)
    }

    /// Get the parameters required by this contract template
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::Contract;
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let params = contract.parameters();
    /// ```
    #[must_use]
    pub fn parameters(&self) -> &Parameters {
        self.template.parameters()
    }

    /// Instantiate the contract with the given arguments
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::Contract;
    ///
    /// let source = "fn main() { assert!(true); }";
    /// let contract = Contract::from_source(source).unwrap();
    /// assert_eq!(contract.source(), source);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// let cmr = compiled.cmr();
    /// assert_eq!(cmr.as_ref().len(), 32);
    /// ```
    #[must_use]
    pub fn cmr(&self) -> simplicityhl::simplicity::Cmr {
        self.inner.commit().cmr()
    }

    /// Generate a taproot address for this contract
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments, elements};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// let address = compiled.address(&elements::AddressParams::ELEMENTS);
    /// assert!(address.to_string().starts_with("ert1p"));
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// let (script, version) = compiled.script_version();
    /// assert!(!script.is_empty());
    /// ```
    #[must_use]
    pub fn script_version(&self) -> (elements::Script, elements::taproot::LeafVersion) {
        let script = elements::script::Script::from(self.cmr().as_ref().to_vec());
        (script, simplicityhl::simplicity::leaf_version())
    }

    /// Satisfy the contract with witness values, producing a satisfied program
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments, WitnessValues};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// let inner = compiled.inner();
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments, WitnessValues};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();
    /// let (program, witness) = satisfied.encode();
    /// assert!(!program.is_empty());
    /// ```
    #[must_use]
    pub fn encode(&self) -> (Vec<u8>, Vec<u8>) {
        self.inner.redeem().to_vec_with_witness()
    }

    /// Get the underlying satisfied program
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Contract, Arguments, WitnessValues};
    ///
    /// let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = contract.instantiate(Arguments::default()).unwrap();
    /// let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();
    /// let inner = satisfied.inner();
    /// ```
    #[must_use]
    pub const fn inner(&self) -> &simplicityhl::SatisfiedProgram {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_source_valid() {
        let contract = Contract::from_source("fn main() { assert!(true); }");
        assert!(contract.is_ok());
    }

    #[test]
    fn test_from_source_invalid_syntax() {
        let contract = Contract::from_source("invalid syntax !!!!");
        assert!(contract.is_err());
    }

    #[test]
    fn test_instantiate_no_params() {
        let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = contract.instantiate(Arguments::default());
        assert!(compiled.is_ok());
    }

    #[test]
    fn test_cmr_deterministic() {
        let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
        let compiled1 = contract.instantiate(Arguments::default()).unwrap();
        let compiled2 = contract.instantiate(Arguments::default()).unwrap();
        assert_eq!(compiled1.cmr(), compiled2.cmr());
    }

    #[test]
    fn test_address_generation() {
        let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = contract.instantiate(Arguments::default()).unwrap();
        let address = compiled.address(&elements::AddressParams::ELEMENTS);
        assert!(address.to_string().starts_with("ert1p"));
    }

    #[test]
    fn test_satisfy_empty_witness() {
        let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = contract.instantiate(Arguments::default()).unwrap();
        let satisfied = compiled.satisfy(WitnessValues::default());
        assert!(satisfied.is_ok());
    }

    #[test]
    fn test_encode() {
        let contract = Contract::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = contract.instantiate(Arguments::default()).unwrap();
        let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();
        let (program, witness) = satisfied.encode();
        assert!(!program.is_empty());
    }

    #[test]
    fn test_source_preservation() {
        let source = "fn main() { assert!(true); }";
        let contract = Contract::from_source(source).unwrap();
        assert_eq!(contract.source(), source);
    }
}
