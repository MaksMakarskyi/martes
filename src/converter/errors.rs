use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConverterError {
    /// Used when some operation failed unexpectedly
    #[error("unexpected error happend during {operation}")]
    UnexpectedErr { operation: &'static str },
}
