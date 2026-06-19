use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    /// Used when a leaf block appeared to be in the place of
    /// a parent for another block
    #[error("{block_type} cannot be used as a contaier block")]
    InvalidContainer { block_type: String },

    /// Used when a block cannot be used as a child for another block.
    ///
    /// Example: a list cannot contain a paragraph as a child;
    /// it can contain only list items.
    #[error("{child} cannot be used as a child of {parent}")]
    InvalidChild { parent: String, child: String },

    /// Used inline content was expected to be raw
    #[error("the content must be raw at this point")]
    ExpectedRawContent,
}
