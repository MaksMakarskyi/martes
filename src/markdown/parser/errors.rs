use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    /// Used when a leaf block appeared to be in the place of
    /// a parent for another block
    #[error("{block_type} cannot be used as a contaier block")]
    InvalidContainer { block_type: String },

    /// Used inline content was expected to be raw
    #[error("the content must be raw at this point")]
    ExpectedRawContent,
}
