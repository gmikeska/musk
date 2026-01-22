//! Program compilation and instantiation

use crate::address::create_taproot_info;
use crate::error::ProgramError;
use elements::taproot::TaprootSpendInfo;
use secp256k1::PublicKey;
use simplicityhl::{Arguments, CompiledProgram, Parameters, TemplateProgram, WitnessValues};
use std::path::Path;
use std::sync::Arc;

/// Address type for Simplicity programs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressType {
    /// Explicit address - amounts and assets are visible on-chain
    #[default]
    Explicit,
    /// Confidential address - amounts and assets are blinded
    Confidential,
}

/// A Simplicity program template with parameterized values
pub struct Program {
    source: Arc<str>,
    template: TemplateProgram,
}

impl Program {
    /// Create a program from source code
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::Program;
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// assert!(program.source().len() > 0);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the source code cannot be parsed.
    pub fn from_source(source: &str) -> Result<Self, ProgramError> {
        let source = Arc::from(source);
        let template =
            TemplateProgram::new(Arc::clone(&source)).map_err(ProgramError::ParseError)?;

        Ok(Self { source, template })
    }

    /// Load a program from a file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use musk::Program;
    ///
    /// let program = Program::from_file("program.simf")?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ProgramError> {
        let source = std::fs::read_to_string(path)?;
        Self::from_source(&source)
    }

    /// Get the parameters required by this program template
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::Program;
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let params = program.parameters();
    /// ```
    #[must_use]
    pub fn parameters(&self) -> &Parameters {
        self.template.parameters()
    }

    /// Instantiate the program with the given arguments
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Program, Arguments};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if instantiation fails or the taproot tree cannot be built.
    pub fn instantiate(&self, arguments: Arguments) -> Result<InstantiatedProgram, ProgramError> {
        let compiled = self
            .template
            .instantiate(arguments, false)
            .map_err(ProgramError::InstantiationError)?;

        let taproot_info = create_taproot_info(&compiled)?;

        Ok(InstantiatedProgram {
            inner: compiled,
            taproot_info,
        })
    }

    /// Get the source code
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::Program;
    ///
    /// let source = "fn main() { assert!(true); }";
    /// let program = Program::from_source(source).unwrap();
    /// assert_eq!(program.source(), source);
    /// ```
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// An instantiated Simplicity program ready for address generation and spending
#[derive(Clone)]
pub struct InstantiatedProgram {
    inner: CompiledProgram,
    taproot_info: TaprootSpendInfo,
}

impl InstantiatedProgram {
    /// Get the commitment Merkle root (CMR) of this program
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Program, Arguments};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
    /// let cmr = compiled.cmr();
    /// assert_eq!(cmr.as_ref().len(), 32);
    /// ```
    #[must_use]
    pub fn cmr(&self) -> simplicityhl::simplicity::Cmr {
        self.inner.commit().cmr()
    }

    /// Generate an explicit taproot address for this program (no blinding)
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Program, Arguments, elements};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
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

    /// Generate a confidential taproot address for this program
    ///
    /// A confidential address includes a blinding public key that enables
    /// confidential transactions where amounts and asset IDs are encrypted.
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Program, Arguments, elements};
    /// use secp256k1::{Secp256k1, SecretKey, PublicKey};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
    ///
    /// // Generate a blinding keypair
    /// let secp = Secp256k1::new();
    /// let blinding_sk = SecretKey::from_slice(&[1u8; 32]).unwrap();
    /// let blinding_pk = PublicKey::from_secret_key(&secp, &blinding_sk);
    ///
    /// let address = compiled.confidential_address(&elements::AddressParams::ELEMENTS, blinding_pk);
    /// // Confidential addresses have a different prefix
    /// ```
    #[must_use]
    pub fn confidential_address(
        &self,
        params: &'static elements::AddressParams,
        blinding_key: PublicKey,
    ) -> elements::Address {
        elements::Address::p2tr(
            &secp256k1::Secp256k1::new(),
            self.taproot_info.internal_key(),
            self.taproot_info.merkle_root(),
            Some(blinding_key),
            params,
        )
    }

    /// Generate a taproot address with an optional blinding key
    ///
    /// This is a convenience method that handles both explicit and confidential addresses.
    #[must_use]
    pub fn address_with_blinder(
        &self,
        params: &'static elements::AddressParams,
        blinding_key: Option<PublicKey>,
    ) -> elements::Address {
        elements::Address::p2tr(
            &secp256k1::Secp256k1::new(),
            self.taproot_info.internal_key(),
            self.taproot_info.merkle_root(),
            blinding_key,
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
    /// use musk::{Program, Arguments};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
    /// let (script, version) = compiled.script_version();
    /// assert!(!script.is_empty());
    /// ```
    #[must_use]
    pub fn script_version(&self) -> (elements::Script, elements::taproot::LeafVersion) {
        let script = elements::script::Script::from(self.cmr().as_ref().to_vec());
        (script, simplicityhl::simplicity::leaf_version())
    }

    /// Satisfy the program with witness values, producing a satisfied program
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Program, Arguments, WitnessValues};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
    /// let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the witness values are invalid or incomplete.
    pub fn satisfy(&self, witness_values: WitnessValues) -> Result<SatisfiedProgram, ProgramError> {
        let satisfied = self
            .inner
            .satisfy(witness_values)
            .map_err(ProgramError::SatisfactionError)?;

        Ok(SatisfiedProgram {
            inner: satisfied,
            taproot_info: self.taproot_info.clone(),
        })
    }

    /// Get the underlying compiled program
    ///
    /// # Examples
    ///
    /// ```
    /// use musk::{Program, Arguments};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
    /// let inner = compiled.inner();
    /// ```
    #[must_use]
    pub const fn inner(&self) -> &CompiledProgram {
        &self.inner
    }
}

/// A satisfied Simplicity program ready to be encoded in a transaction witness
pub struct SatisfiedProgram {
    inner: simplicityhl::SatisfiedProgram,
    taproot_info: TaprootSpendInfo,
}

impl SatisfiedProgram {
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
    /// use musk::{Program, Arguments, WitnessValues};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
    /// let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();
    /// let (program_bytes, witness) = satisfied.encode();
    /// assert!(!program_bytes.is_empty());
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
    /// use musk::{Program, Arguments, WitnessValues};
    ///
    /// let program = Program::from_source("fn main() { assert!(true); }").unwrap();
    /// let compiled = program.instantiate(Arguments::default()).unwrap();
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_from_source_valid() {
        let program = Program::from_source("fn main() { assert!(true); }");
        assert!(program.is_ok());
    }

    #[test]
    fn test_from_source_invalid_syntax() {
        let result = Program::from_source("invalid syntax !!!!");
        assert!(result.is_err());
        match result {
            Err(ProgramError::ParseError(_)) => {}
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_from_file_valid() {
        let source = "fn main() { assert!(true); }";
        let mut temp_file = NamedTempFile::with_suffix(".simf").unwrap();
        temp_file.write_all(source.as_bytes()).unwrap();

        let program = Program::from_file(temp_file.path());
        assert!(program.is_ok());
        assert_eq!(program.unwrap().source(), source);
    }

    #[test]
    fn test_from_file_not_found() {
        let result = Program::from_file("/nonexistent/path/program.simf");
        assert!(result.is_err());
        match result {
            Err(ProgramError::IoError(_)) => {}
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_from_file_invalid_syntax() {
        let source = "this is not valid simplicity!!!";
        let mut temp_file = NamedTempFile::with_suffix(".simf").unwrap();
        temp_file.write_all(source.as_bytes()).unwrap();

        let result = Program::from_file(temp_file.path());
        assert!(result.is_err());
        match result {
            Err(ProgramError::ParseError(_)) => {}
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_instantiate_no_params() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default());
        assert!(compiled.is_ok());
    }

    #[test]
    fn test_parameters_access() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        // Just verify we can access parameters without error
        let _params = program.parameters();
    }

    #[test]
    fn test_cmr_deterministic() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled1 = program.instantiate(Arguments::default()).unwrap();
        let compiled2 = program.instantiate(Arguments::default()).unwrap();
        assert_eq!(compiled1.cmr(), compiled2.cmr());
    }

    #[test]
    fn test_address_generation() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();
        let address = compiled.address(&elements::AddressParams::ELEMENTS);
        assert!(address.to_string().starts_with("ert1p"));
    }

    #[test]
    fn test_address_generation_different_networks() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();

        // Regtest
        let regtest_addr = compiled.address(&elements::AddressParams::ELEMENTS);
        assert!(regtest_addr.to_string().starts_with("ert1p"));

        // Liquid
        let liquid_addr = compiled.address(&elements::AddressParams::LIQUID);
        assert!(
            liquid_addr.to_string().starts_with("ex1p")
                || liquid_addr.to_string().starts_with("lq1p")
        );

        // Testnet
        let testnet_addr = compiled.address(&elements::AddressParams::LIQUID_TESTNET);
        assert!(
            testnet_addr.to_string().starts_with("tex1p")
                || testnet_addr.to_string().starts_with("tlq1p")
        );
    }

    #[test]
    fn test_satisfy_empty_witness() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();
        let satisfied = compiled.satisfy(WitnessValues::default());
        assert!(satisfied.is_ok());
    }

    #[test]
    fn test_encode() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();
        let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();
        let (program_bytes, _witness) = satisfied.encode();
        assert!(!program_bytes.is_empty());
    }

    #[test]
    fn test_encode_deterministic() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();

        let satisfied1 = compiled.satisfy(WitnessValues::default()).unwrap();
        let (prog1, wit1) = satisfied1.encode();

        let satisfied2 = compiled.satisfy(WitnessValues::default()).unwrap();
        let (prog2, wit2) = satisfied2.encode();

        assert_eq!(prog1, prog2);
        assert_eq!(wit1, wit2);
    }

    #[test]
    fn test_source_preservation() {
        let source = "fn main() { assert!(true); }";
        let program = Program::from_source(source).unwrap();
        assert_eq!(program.source(), source);
    }

    #[test]
    fn test_taproot_info() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();

        let taproot_info = compiled.taproot_info();
        assert_eq!(taproot_info.internal_key().serialize().len(), 32);
        assert!(taproot_info.merkle_root().is_some());
    }

    #[test]
    fn test_script_version() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();

        let (script, version) = compiled.script_version();

        // Script should be 32 bytes (CMR)
        assert_eq!(script.len(), 32);
        assert_eq!(script.as_bytes(), compiled.cmr().as_ref());

        // Version should be Simplicity leaf version
        assert_eq!(version, simplicityhl::simplicity::leaf_version());
    }

    #[test]
    fn test_inner_access() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();

        // Should be able to access inner CompiledProgram
        let inner = compiled.inner();
        assert!(std::mem::size_of_val(inner) > 0);
    }

    #[test]
    fn test_satisfied_inner_access() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();
        let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();

        // Should be able to access inner SatisfiedProgram
        let inner = satisfied.inner();
        assert!(std::mem::size_of_val(inner) > 0);
    }

    #[test]
    fn test_satisfied_taproot_info() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();
        let satisfied = compiled.satisfy(WitnessValues::default()).unwrap();

        // Satisfied program should have same taproot info
        let taproot_info = satisfied.taproot_info();
        assert_eq!(
            taproot_info.internal_key(),
            compiled.taproot_info().internal_key()
        );
    }

    #[test]
    fn test_instantiated_program_clone() {
        let program = Program::from_source("fn main() { assert!(true); }").unwrap();
        let compiled = program.instantiate(Arguments::default()).unwrap();

        let cloned = compiled.clone();

        assert_eq!(compiled.cmr(), cloned.cmr());
        assert_eq!(
            compiled.address(&elements::AddressParams::ELEMENTS),
            cloned.address(&elements::AddressParams::ELEMENTS)
        );
    }
}
