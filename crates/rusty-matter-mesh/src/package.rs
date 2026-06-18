use crate::{MatterMeshError, MeshCoordinateMap, MeshSourceDescriptor, TriangleMeshSurface};

/// Schema ID for packaged mesh coordinate-map artifacts.
pub const MESH_COORDINATE_MAP_PACKAGE_SCHEMA_ID: &str =
    "rusty.matter.mesh.coordinate_map_package.v1";

/// A source mesh, canonical Matter surface, and coordinate map bound together.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCoordinateMapPackage {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable package identifier.
    pub package_id: String,
    /// Provenance and source interpretation metadata.
    pub source: MeshSourceDescriptor,
    /// Canonical Matter triangle surface.
    pub surface: TriangleMeshSurface,
    /// Coordinate map derived from `surface`.
    pub coordinate_map: MeshCoordinateMap,
    /// Additional package notes.
    pub notes: Vec<String>,
}

impl MeshCoordinateMapPackage {
    /// Creates a coordinate-map package from validated parts.
    #[must_use]
    pub fn new(
        package_id: impl Into<String>,
        source: MeshSourceDescriptor,
        surface: TriangleMeshSurface,
        coordinate_map: MeshCoordinateMap,
    ) -> Self {
        Self {
            schema_id: MESH_COORDINATE_MAP_PACKAGE_SCHEMA_ID.to_owned(),
            package_id: package_id.into(),
            source,
            surface,
            coordinate_map,
            notes: Vec::new(),
        }
    }

    /// Validates package, source, surface, and coordinate-map consistency.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the package is internally inconsistent.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != MESH_COORDINATE_MAP_PACKAGE_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: MESH_COORDINATE_MAP_PACKAGE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.package_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyMeshCoordinateMapPackageId);
        }
        self.source.validate()?;
        self.surface.validate()?;
        if !self.coordinate_map.is_valid_for_surface(&self.surface) {
            return Err(MatterMeshError::InvalidMeshCoordinateMapPackage(
                "coordinate_map must match package surface",
            ));
        }
        if self.notes.iter().any(|note| !note.is_ascii()) {
            return Err(MatterMeshError::InvalidMeshCoordinateMapPackage(
                "notes must be ASCII for portable fixtures",
            ));
        }
        Ok(())
    }
}
