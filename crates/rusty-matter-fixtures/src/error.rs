use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum CliError {
    Usage(String),
    UnknownCommand(String),
    UnknownOption(String),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialize(serde_json::Error),
    Adf(rusty_matter_adf::AdfError),
    Field(rusty_matter_fields::MatterFieldError),
    Sdf(rusty_matter_sdf::SdfError),
    Mesh(rusty_matter_mesh::MatterMeshError),
    Particle(rusty_matter_particles::ParticleError),
    MissingColliderContact,
    ExpectedRejection {
        fixture_id: String,
    },
    UnexpectedRejection {
        expected: String,
        actual: String,
    },
    FixtureMismatch {
        path: PathBuf,
        expected: String,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => formatter.write_str(message),
            Self::UnknownCommand(command) => write!(formatter, "unknown command: {command}"),
            Self::UnknownOption(option) => write!(formatter, "unknown option: {option}"),
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Serialize(error) => write!(formatter, "failed to serialize fixture: {error}"),
            Self::Adf(error) => write!(formatter, "failed to build ADF fixture: {error}"),
            Self::Field(error) => write!(formatter, "failed to build field fixture: {error}"),
            Self::Sdf(error) => write!(formatter, "failed to build SDF fixture: {error}"),
            Self::Mesh(error) => write!(formatter, "failed to build mesh fixture: {error}"),
            Self::Particle(error) => write!(formatter, "failed to build particle fixture: {error}"),
            Self::MissingColliderContact => {
                formatter.write_str("failed to build mesh fixture: collider contact missing")
            }
            Self::ExpectedRejection { fixture_id } => {
                write!(formatter, "damaged fixture {fixture_id} did not reject")
            }
            Self::UnexpectedRejection { expected, actual } => write!(
                formatter,
                "damaged fixture rejected with {actual}, expected {expected}"
            ),
            Self::FixtureMismatch { path, expected } => write!(
                formatter,
                "fixture mismatch at {}; expected:\n{expected}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for CliError {}
