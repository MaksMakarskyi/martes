use super::constants::{SUPPORTED_INPUT_EXTENSIONS, SUPPORTED_OUTPUT_EXTENSIONS};
use super::errors::ConfigError;
use std::path::Path;

pub fn is_in_ext_supported(ext: &str) -> bool {
    SUPPORTED_INPUT_EXTENSIONS.contains(&ext)
}

pub fn is_out_ext_supported(ext: &str) -> bool {
    SUPPORTED_OUTPUT_EXTENSIONS.contains(&ext)
}

pub fn get_extension(path: &Path) -> Result<&str, ConfigError> {
    path.extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| ConfigError::InvalidExtension {
            path: path.to_owned(),
        })
}
