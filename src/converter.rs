pub mod errors;

use super::config;
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Debug)]
pub struct Converter {
    input: PathBuf,
    output: PathBuf,
    overwrite: bool,
}

impl Converter {
    pub fn open_file(self) -> String {
        let mut file = File::open(self.input).unwrap();

        let mut res = String::new();
        file.read_to_string(&mut res).unwrap();

        res
    }
}

impl From<config::Config> for Converter {
    fn from(value: config::Config) -> Self {
        Converter {
            input: value.input,
            output: value.output,
            overwrite: value.overwrite,
        }
    }
}
