use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum CliError {
    Usage(String),
    UnknownOption(String),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialize(serde_json::Error),
    CatalogMismatch {
        schema_path: PathBuf,
        output: String,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => formatter.write_str(message),
            Self::UnknownOption(option) => write!(formatter, "unknown option: {option}"),
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Serialize(error) => write!(formatter, "failed to serialize catalog: {error}"),
            Self::CatalogMismatch {
                schema_path,
                output,
            } => write!(
                formatter,
                "schema catalog mismatch at {}; expected:\n{output}",
                schema_path.display()
            ),
        }
    }
}

impl std::error::Error for CliError {}
