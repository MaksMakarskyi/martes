use martes::{config, converter};

fn main() {
    if let Err(err) = run() {
        println!("{:?}", err);
    }
}

fn run() -> Result<(), &'static str> {
    let config = match config::Config::build() {
        Ok(cfg) => cfg,
        Err(err) => panic!("{}", err),
        // Err(_) => return Err("some err happend"),
    };

    let c = converter::Converter::from(config);

    match c.convert() {
        Ok(()) => Ok(()),
        Err(err) => panic!("{}", err),
    }
}
