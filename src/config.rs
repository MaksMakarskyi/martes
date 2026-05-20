mod constants;
mod utils;

pub mod errors;

use clap::Parser;
use errors::{ConfigError, FileType};
use std::path::PathBuf;
use utils::{get_extension, is_in_ext_supported, is_out_ext_supported};

/// Martes is a simple CLI tool for converting Markdown files into HTML format
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct RawConfig {
    /// Path to the input file
    ///
    /// Supported formats: Markdown
    ///
    /// Supported file extensions: .md, .txt
    input: PathBuf,

    /// Path to the output file
    ///
    /// Supported formats: HTML
    ///
    /// Supported file extensions: .html
    ///
    /// When not specified, the name and location of input file
    /// will be used. For example, /path/file.md will be stored
    /// under /path/file.html
    output: Option<PathBuf>,

    /// Overwrite existing file
    ///
    /// Examples:
    ///
    /// - when set to `false`, name conflict is resolved by adding timestamp to the file name
    ///
    /// - when set to `true`, overwrites the existing file
    #[arg(short, long)]
    overwrite: bool,
}

pub struct Config {
    pub input: PathBuf,
    pub output: PathBuf,
    pub overwrite: bool,
}

impl From<RawConfig> for Config {
    fn from(value: RawConfig) -> Self {
        let output = value.output.unwrap_or(value.input.with_extension("html"));

        Self {
            input: value.input,
            output: output,
            overwrite: value.overwrite,
        }
    }
}

impl Config {
    /// Creates a new Config instance and validates the CLI args
    pub fn build() -> Result<Self, ConfigError> {
        let raw_config = RawConfig::parse();
        Self::from(raw_config).validate()
    }

    fn validate(self) -> Result<Self, ConfigError> {
        if !self.input.exists() {
            return Err(ConfigError::MissingFile { path: self.input });
        }

        if self.output.exists() && !self.overwrite {
            return Err(ConfigError::FileAlreadyExist { path: self.output });
        }

        let input_file_ext = get_extension(&self.input)?;
        let output_file_ext = get_extension(&self.output)?;

        if !is_in_ext_supported(input_file_ext) {
            return Err(ConfigError::UnsupportedExtension {
                extension: String::from(input_file_ext),
                file_type: FileType::Input,
            });
        }
        if !is_out_ext_supported(output_file_ext) {
            return Err(ConfigError::UnsupportedExtension {
                extension: String::from(output_file_ext),
                file_type: FileType::Output,
            });
        }

        if input_file_ext == output_file_ext {
            return Err(ConfigError::SameExtension);
        }

        Ok(self)
    }
}
