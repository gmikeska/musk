//! Address generation and taproot utilities

use crate::error::ContractError;
use crate::util::default_internal_key;
use elements::taproot::{TaprootBuilder, TaprootSpendInfo};
use secp256k1::Secp256k1;
use simplicityhl::CompiledProgram;

/// Create taproot spend info for a compiled contract
pub fn create_taproot_info(compiled: &CompiledProgram) -> Result<TaprootSpendInfo, ContractError> {
    let internal_key = default_internal_key();
    let builder = TaprootBuilder::new();

    let script = elements::script::Script::from(compiled.commit().cmr().as_ref().to_vec());
    let version = simplicityhl::simplicity::leaf_version();

    let builder = builder
        .add_leaf_with_ver(0, script, version)
        .map_err(|e| ContractError::TaprootError(e.to_string()))?;

    builder
        .finalize(&Secp256k1::new(), internal_key)
        .map_err(|e| ContractError::TaprootError(e.to_string()))
}
