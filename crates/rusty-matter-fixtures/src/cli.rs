use std::path::{Path, PathBuf};

use crate::artifact::build_fixture_artifacts;
use crate::error::CliError;

pub(crate) fn run(args: Vec<String>) -> Result<String, CliError> {
    let options = Options::parse(args)?;
    let artifacts = build_fixture_artifacts()?;

    match options.command {
        Command::Write => {
            for artifact in &artifacts {
                artifact.write(&options.repo_root)?;
            }
            Ok(format!("wrote {} fixture artifacts", artifacts.len()))
        }
        Command::Validate => {
            for artifact in &artifacts {
                artifact.validate(&options.repo_root)?;
            }
            Ok(format!("fixtures validate: {} artifacts", artifacts.len()))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Options {
    command: Command,
    repo_root: PathBuf,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut args = args.into_iter();
        let Some(command) = args.next() else {
            return Err(CliError::Usage(usage()));
        };
        let command = match command.as_str() {
            "validate" => Command::Validate,
            "write" => Command::Write,
            "-h" | "--help" => return Err(CliError::Usage(usage())),
            other => return Err(CliError::UnknownCommand(other.to_owned())),
        };

        let mut repo_root = default_repo_root();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--repo-root" => {
                    let Some(value) = args.next() else {
                        return Err(CliError::Usage("--repo-root requires a value".to_owned()));
                    };
                    repo_root = PathBuf::from(value);
                }
                "-h" | "--help" => return Err(CliError::Usage(usage())),
                other => return Err(CliError::UnknownOption(other.to_owned())),
            }
        }

        Ok(Self { command, repo_root })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Validate,
    Write,
}

fn default_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives under repo/crates/name")
        .to_path_buf()
}

fn usage() -> String {
    "usage: rusty-matter-fixtures <validate|write> [--repo-root <path>]".to_owned()
}
