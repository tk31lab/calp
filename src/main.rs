use calp::{run, Config};
use clap::Parser;

fn main() {
    if let Err(e) = run(Config::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
