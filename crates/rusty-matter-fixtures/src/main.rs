//! Deterministic Matter fixture validation CLI.

use std::env;
use std::process::ExitCode;

mod artifact;
mod cli;
mod damaged;
mod error;
mod mesh;
mod particles;
mod sdf;
mod summary;

fn main() -> ExitCode {
    match cli::run(env::args().skip(1).collect()) {
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
