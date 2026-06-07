use std::fs;
use std::path::{Path, PathBuf};

use crate::catalog::SchemaCatalog;
use crate::error::CliError;

pub(crate) fn run(args: Vec<String>) -> Result<String, CliError> {
    let options = Options::parse(args)?;
    let catalog = SchemaCatalog::current();
    let output = serde_json::to_string_pretty(&catalog).map_err(CliError::Serialize)?;
    let schema_path = options.repo_root.join("schemas/catalog.json");

    if options.check {
        let existing = fs::read_to_string(&schema_path).map_err(|source| CliError::Io {
            path: schema_path.clone(),
            source,
        })?;
        if existing.trim_end() == output.trim_end() {
            Ok("schema catalog matches".to_owned())
        } else {
            Err(CliError::CatalogMismatch {
                schema_path,
                output,
            })
        }
    } else {
        fs::write(&schema_path, format!("{output}\n")).map_err(|source| CliError::Io {
            path: schema_path.clone(),
            source,
        })?;
        Ok(format!("wrote {}", schema_path.display()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Options {
    repo_root: PathBuf,
    check: bool,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut args = args.into_iter();
        let Some(command) = args.next() else {
            return Err(CliError::Usage(usage()));
        };
        if command != "export" {
            return Err(CliError::Usage(usage()));
        }

        let mut repo_root = default_repo_root();
        let mut check = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--check" => check = true,
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

        Ok(Self { repo_root, check })
    }
}

fn default_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives under repo/crates/name")
        .to_path_buf()
}

fn usage() -> String {
    "usage: rusty-matter-schema export [--check] [--repo-root <path>]".to_owned()
}
