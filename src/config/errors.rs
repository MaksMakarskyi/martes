use core::fmt;
use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    /// Used when both, input and output files have the same extension -
    /// no-op case
    #[error("input and output file must have different extensions")]
    SameExtension,

    /// Used when the specified file extansion is unsopported for particular
    /// [`FileType`]
    #[error("unsupported {file_type} file extension: {extension}")]
    UnsupportedExtension {
        extension: String,
        file_type: FileType,
    },

    /// Used when the specified file extansion is missing or invalid
    #[error("failed to read file extension from path: {path}")]
    InvalidExtension { path: PathBuf },

    /// Used when [`super::Config::input`] is missing
    #[error("file does not exist: {path}")]
    MissingFile { path: PathBuf },

    /// Used when output file exist but the [`super::Config::overwrite`] parameter
    /// is `false`
    #[error("file does already exist: {path}")]
    FileAlreadyExist { path: PathBuf },

    /// Used when some operation failed unexpectedly
    #[error("unexpected error happend during {operation}")]
    UnexpectedErr { operation: &'static str },
}

#[derive(Debug)]
pub enum FileType {
    Input,
    Output,
}

impl Display for FileType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FileType::Input => "input",
            FileType::Output => "output",
        })
    }
}
