use crate::{Bounds3, MatterModelError, MatterSchemaId, Vec3};

/// Schema ID for triangle mesh snapshots.
pub const TRIANGLE_MESH_SCHEMA_ID: &str = "rusty.matter.mesh.triangle_mesh.v1";

/// Triangle mesh payload.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TriangleMeshSnapshot {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable mesh identifier.
    pub mesh_id: String,
    /// Mesh vertex positions.
    pub positions: Vec<Vec3>,
    /// Triangle indices.
    pub indices: Vec<[u32; 3]>,
}

impl TriangleMeshSnapshot {
    /// Creates a triangle mesh snapshot.
    #[must_use]
    pub fn new(mesh_id: impl Into<String>, positions: Vec<Vec3>, indices: Vec<[u32; 3]>) -> Self {
        Self {
            schema_id: TRIANGLE_MESH_SCHEMA_ID.to_owned(),
            mesh_id: mesh_id.into(),
            positions,
            indices,
        }
    }

    /// Validates mesh structure.
    ///
    /// # Errors
    ///
    /// Returns [`MatterModelError`] when the mesh is empty, non-finite, or has
    /// invalid indices.
    pub fn validate(&self) -> Result<(), MatterModelError> {
        MatterSchemaId::new(self.schema_id.clone())?;
        if self.schema_id != TRIANGLE_MESH_SCHEMA_ID {
            return Err(MatterModelError::UnexpectedSchema {
                expected: TRIANGLE_MESH_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.mesh_id.trim().is_empty() {
            return Err(MatterModelError::EmptyMeshId);
        }
        if self.positions.is_empty() {
            return Err(MatterModelError::EmptyPointSet);
        }
        if self.indices.is_empty() {
            return Err(MatterModelError::EmptyIndexSet);
        }
        for (index, point) in self.positions.iter().copied().enumerate() {
            if !point.is_finite() {
                return Err(MatterModelError::NonFinitePoint { index });
            }
        }
        let vertex_count = self.positions.len();
        for (triangle_index, triangle) in self.indices.iter().copied().enumerate() {
            let [a, b, c] = triangle;
            if a == b || b == c || a == c {
                return Err(MatterModelError::DegenerateTriangle { triangle_index });
            }
            for vertex_index in triangle {
                let as_usize = usize::try_from(vertex_index).map_err(|_| {
                    MatterModelError::IndexOutOfRange {
                        triangle_index,
                        vertex_index,
                        vertex_count,
                    }
                })?;
                if as_usize >= vertex_count {
                    return Err(MatterModelError::IndexOutOfRange {
                        triangle_index,
                        vertex_index,
                        vertex_count,
                    });
                }
            }
        }
        Ok(())
    }

    /// Returns mesh bounds.
    ///
    /// # Errors
    ///
    /// Returns [`MatterModelError`] when positions are empty or non-finite.
    pub fn bounds(&self) -> Result<Bounds3, MatterModelError> {
        Bounds3::from_points(&self.positions)
    }
}
