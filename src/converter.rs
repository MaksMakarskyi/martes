pub mod errors;

use super::config;
use super::markdown::parser;
use errors::ConverterError;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub struct Converter {
    input: PathBuf,
    output: PathBuf,
    overwrite: bool,
}

impl Converter {
    pub fn convert(self) -> Result<(), ConverterError> {
        let mut input_file = File::open(&self.input).unwrap();
        let mut res = String::new();
        input_file.read_to_string(&mut res).unwrap();

        let md_doc = parser::parse(&res)?;
        println!("{:?}", md_doc);
        self.save_file(&md_doc.to_html())
    }

    fn save_file(&self, output: &str) -> Result<(), ConverterError> {
        let mut output_file: File;

        if self.overwrite {
            output_file = match File::create(&self.output) {
                Ok(f) => f,
                Err(_) => {
                    return Err(ConverterError::UnexpectedErr {
                        operation: "opening output file in overwrite mode",
                    });
                }
            };
        } else {
            output_file = match File::create_new(&self.output) {
                Ok(f) => f,
                Err(_) => {
                    return Err(ConverterError::UnexpectedErr {
                        operation: "opening output file",
                    });
                }
            };
        }

        match output_file.write_all(output.as_bytes()) {
            Ok(_) => return Ok(()),
            Err(_) => {
                return Err(ConverterError::UnexpectedErr {
                    operation: "writing outout to the file",
                });
            }
        }
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
