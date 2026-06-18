use crate::MatterMeshError;

/// Schema ID for mesh source descriptors.
pub const MESH_SOURCE_DESCRIPTOR_SCHEMA_ID: &str = "rusty.matter.mesh.source_descriptor.v1";

/// Provenance and interpretation metadata for an imported or generated mesh.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshSourceDescriptor {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable source identifier.
    pub source_id: String,
    /// Source URI, path label, or procedural origin.
    pub source_uri: String,
    /// Declared format such as `stl`, `glb`, `obj`, `procedural`, or `matter.surface`.
    pub source_format: String,
    /// Stable source content hash or deterministic procedural content key.
    pub source_hash: String,
    /// License label for the source mesh or generated surface.
    pub license: String,
    /// Human-readable attribution or origin statement.
    pub attribution: String,
    /// Unit scale applied when converting source units to meters.
    pub unit_scale_to_meters: f32,
    /// Axis convention after import, for example `x_right_y_up_z_forward`.
    pub axis_convention: String,
    /// Additional source or review notes.
    pub notes: Vec<String>,
}

impl MeshSourceDescriptor {
    /// Creates a source descriptor with default Matter axis convention.
    #[must_use]
    pub fn new(
        source_id: impl Into<String>,
        source_uri: impl Into<String>,
        source_format: impl Into<String>,
        source_hash: impl Into<String>,
        license: impl Into<String>,
        attribution: impl Into<String>,
    ) -> Self {
        Self {
            schema_id: MESH_SOURCE_DESCRIPTOR_SCHEMA_ID.to_owned(),
            source_id: source_id.into(),
            source_uri: source_uri.into(),
            source_format: source_format.into(),
            source_hash: source_hash.into(),
            license: license.into(),
            attribution: attribution.into(),
            unit_scale_to_meters: 1.0,
            axis_convention: "x_right_y_up_z_forward".to_owned(),
            notes: Vec::new(),
        }
    }

    /// Validates source descriptor metadata.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when required source metadata is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != MESH_SOURCE_DESCRIPTOR_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: MESH_SOURCE_DESCRIPTOR_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.source_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyMeshSourceId);
        }
        if self.source_format.trim().is_empty() {
            return Err(MatterMeshError::InvalidMeshSourceDescriptor(
                "source_format must not be empty",
            ));
        }
        if self.source_hash.trim().is_empty() {
            return Err(MatterMeshError::InvalidMeshSourceDescriptor(
                "source_hash must not be empty",
            ));
        }
        if self.license.trim().is_empty() {
            return Err(MatterMeshError::InvalidMeshSourceDescriptor(
                "license must not be empty",
            ));
        }
        if self.attribution.trim().is_empty() {
            return Err(MatterMeshError::InvalidMeshSourceDescriptor(
                "attribution must not be empty",
            ));
        }
        if !self.unit_scale_to_meters.is_finite() || self.unit_scale_to_meters <= 0.0 {
            return Err(MatterMeshError::InvalidMeshSourceDescriptor(
                "unit_scale_to_meters must be finite and positive",
            ));
        }
        if self.axis_convention.trim().is_empty() {
            return Err(MatterMeshError::InvalidMeshSourceDescriptor(
                "axis_convention must not be empty",
            ));
        }
        if self.notes.iter().any(|note| !note.is_ascii()) {
            return Err(MatterMeshError::InvalidMeshSourceDescriptor(
                "notes must be ASCII for portable fixtures",
            ));
        }
        Ok(())
    }
}
