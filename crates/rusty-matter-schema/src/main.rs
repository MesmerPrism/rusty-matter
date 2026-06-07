//! Deterministic schema catalog export CLI.

mod catalog;
mod cli;
mod error;

use std::env;
use std::process::ExitCode;

use cli::run;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
