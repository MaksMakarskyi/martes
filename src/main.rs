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

    println!("{:?}", &c);
    println!("\n{}\n", &c.open_file());

    Ok(())
}
