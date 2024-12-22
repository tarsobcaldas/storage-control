pub mod inventory;
pub mod product;
pub mod repl;
pub mod warehouse;
pub mod test;

use crate::repl::{Cli, run};
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    match run(cli) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
