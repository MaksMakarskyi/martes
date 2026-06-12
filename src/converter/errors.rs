use super::parser::errors;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConverterError {
    /// Used when some operation failed unexpectedly
    #[error("unexpected error happend during {operation}")]
    UnexpectedErr { operation: &'static str },

    /// Used when some operation failed unexpectedly
    #[error("file with the path '{path}' already exists")]
    FileAlreadyExist { path: &'static str },

    /// Used when parser failed internally
    #[error("failed to parse input: {0}")]
    Parse(#[from] errors::ParserError),
}
