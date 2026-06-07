use core::fmt;
use core::str::FromStr;

use crate::MatterModelError;

/// A lowercase dotted identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DottedId(String);

impl DottedId {
    /// Parses and validates a dotted identifier.
    ///
    /// # Errors
    ///
    /// Returns [`MatterModelError`] when the value does not match the dotted-id
    /// grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, MatterModelError> {
        let value = value.into();
        validate_dotted_id(&value)?;
        Ok(Self(value))
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DottedId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DottedId {
    type Err = MatterModelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for DottedId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for DottedId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// A schema identifier following `rusty.matter.<family>.<name>.v<major>`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MatterSchemaId(DottedId);

impl MatterSchemaId {
    /// Parses and validates a Matter schema identifier.
    ///
    /// # Errors
    ///
    /// Returns [`MatterModelError`] when the value is not a valid Matter schema
    /// ID.
    pub fn new(value: impl Into<String>) -> Result<Self, MatterModelError> {
        let value = value.into();
        let dotted = DottedId::new(value.clone())?;
        let parts = value.split('.').collect::<Vec<_>>();

        if parts.len() < 5 || parts[0] != "rusty" || parts[1] != "matter" {
            return Err(MatterModelError::InvalidSchemaPrefix(value));
        }

        let version = parts.last().copied().unwrap_or_default();
        let digits = version.strip_prefix('v').unwrap_or_default();
        if digits.is_empty()
            || digits.starts_with('0')
            || !digits.chars().all(|c| c.is_ascii_digit())
        {
            return Err(MatterModelError::InvalidSchemaVersion(value));
        }

        Ok(Self(dotted))
    }

    /// Returns the schema identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for MatterSchemaId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MatterSchemaId {
    type Err = MatterModelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MatterSchemaId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MatterSchemaId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

fn validate_dotted_id(value: &str) -> Result<(), MatterModelError> {
    if value.is_empty() {
        return Err(MatterModelError::EmptyId);
    }

    for segment in value.split('.') {
        if segment.is_empty() {
            return Err(MatterModelError::EmptyIdSegment);
        }
        let first = segment.chars().next().expect("segment is non-empty");
        let last = segment.chars().last().expect("segment is non-empty");
        if !is_id_edge(first) || !is_id_edge(last) {
            return Err(MatterModelError::InvalidIdSegmentEdge(segment.to_owned()));
        }
        for character in segment.chars() {
            if !is_id_character(character) {
                return Err(MatterModelError::InvalidIdCharacter(character));
            }
        }
    }

    Ok(())
}

fn is_id_edge(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit()
}

fn is_id_character(character: char) -> bool {
    is_id_edge(character) || character == '_' || character == '-'
}
