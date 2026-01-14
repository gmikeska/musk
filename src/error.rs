//! Error types for musk operations

use thiserror::Error;

/// Errors that can occur during program operations
#[derive(Debug, Error)]
pub enum ProgramError {
    #[error("Failed to parse program source: {0}")]
    ParseError(String),

    #[error("Failed to compile program: {0}")]
    CompileError(String),

    #[error("Failed to instantiate program: {0}")]
    InstantiationError(String),

    #[error("Failed to satisfy program: {0}")]
    SatisfactionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid taproot configuration: {0}")]
    TaprootError(String),
}

/// Errors that can occur during spending operations
#[derive(Debug, Error)]
pub enum SpendError {
    #[error("Failed to build transaction: {0}")]
    BuildError(String),

    #[error("Failed to compute sighash: {0}")]
    SighashError(String),

    #[error("Failed to finalize transaction: {0}")]
    FinalizationError(String),

    #[error("Invalid UTXO: {0}")]
    InvalidUtxo(String),

    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),

    #[error("Type inference error: {0}")]
    TypeInferenceError(String),
}
