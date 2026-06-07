use rusty_matter_model::Vec3;

use crate::{
    sample_mesh_surface_points, MatterMeshError, MeshSurfaceSampleConfig, MeshSurfaceSampleSet,
};

/// Schema ID for dynamic triangle mesh surfaces.
pub const TRIANGLE_MESH_SURFACE_SCHEMA_ID: &str = "rusty.matter.mesh.surface.v1";
/// Schema ID for dynamic triangle mesh topology keys.
pub const MESH_SURFACE_TOPOLOGY_KEY_SCHEMA_ID: &str = "rusty.matter.mesh.surface_topology_key.v1";

/// Dynamic triangle mesh surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TriangleMeshSurface {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable surface identifier.
    pub surface_id: String,
    /// Vertex positions.
    pub positions: Vec<Vec3>,
    /// Triangle vertex indices.
    pub triangles: Vec<[u32; 3]>,
}

impl TriangleMeshSurface {
    /// Creates a surface.
    #[must_use]
    pub fn new(
        surface_id: impl Into<String>,
        positions: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
    ) -> Self {
        Self {
            schema_id: TRIANGLE_MESH_SURFACE_SCHEMA_ID.to_owned(),
            surface_id: surface_id.into(),
            positions,
            triangles,
        }
    }

    /// Returns the vertex count.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Returns the triangle count.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Returns the valid triangle surface area.
    #[must_use]
    pub fn surface_area(&self) -> f32 {
        self.triangles
            .iter()
            .filter_map(|triangle| triangle_area(&self.positions, *triangle))
            .sum()
    }

    /// Returns a topology key for this surface.
    #[must_use]
    pub fn topology_key(&self) -> MeshSurfaceTopologyKey {
        MeshSurfaceTopologyKey::from_surface(self)
    }

    /// Samples deterministic surface coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the surface or sampling config is
    /// invalid.
    pub fn sample_points(
        &self,
        config: &MeshSurfaceSampleConfig,
    ) -> Result<MeshSurfaceSampleSet, MatterMeshError> {
        sample_mesh_surface_points(self, config)
    }

    /// Validates surface metadata and geometry.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the surface is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != TRIANGLE_MESH_SURFACE_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: TRIANGLE_MESH_SURFACE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.surface_id.trim().is_empty() {
            return Err(MatterMeshError::EmptySurfaceId);
        }
        if self.positions.is_empty() {
            return Err(MatterMeshError::InvalidSurface(
                "surface must contain positions",
            ));
        }
        if self.triangles.is_empty() {
            return Err(MatterMeshError::InvalidSurface(
                "surface must contain triangles",
            ));
        }
        for (index, position) in self.positions.iter().copied().enumerate() {
            if !position.is_finite() {
                return Err(MatterMeshError::NonFinitePosition { index });
            }
        }
        let mut valid_area = 0.0;
        for (triangle_index, triangle) in self.triangles.iter().copied().enumerate() {
            let [a, b, c] = triangle;
            if a == b || b == c || a == c {
                return Err(MatterMeshError::DegenerateTriangle { triangle_index });
            }
            for vertex_index in triangle {
                let as_usize = usize::try_from(vertex_index).map_err(|_| {
                    MatterMeshError::IndexOutOfRange {
                        triangle_index,
                        vertex_index,
                        vertex_count: self.positions.len(),
                    }
                })?;
                if as_usize >= self.positions.len() {
                    return Err(MatterMeshError::IndexOutOfRange {
                        triangle_index,
                        vertex_index,
                        vertex_count: self.positions.len(),
                    });
                }
            }
            valid_area += triangle_area(&self.positions, [a, b, c]).unwrap_or(0.0);
        }
        if valid_area <= 1.0e-9 {
            return Err(MatterMeshError::InvalidSurface(
                "surface area must be positive",
            ));
        }
        Ok(())
    }
}

/// Stable topology identity for a dynamic mesh surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshSurfaceTopologyKey {
    /// Schema identifier.
    pub schema_id: String,
    /// Number of surface vertices.
    pub vertex_count: usize,
    /// Number of surface triangles.
    pub triangle_count: usize,
    /// Stable FNV-1a hash of triangle indices.
    pub index_hash: u64,
}

impl MeshSurfaceTopologyKey {
    /// Creates a topology key from a surface.
    #[must_use]
    pub fn from_surface(surface: &TriangleMeshSurface) -> Self {
        Self {
            schema_id: MESH_SURFACE_TOPOLOGY_KEY_SCHEMA_ID.to_owned(),
            vertex_count: surface.positions.len(),
            triangle_count: surface.triangles.len(),
            index_hash: mesh_surface_index_hash(&surface.triangles),
        }
    }
}

pub(crate) fn triangle_area(positions: &[Vec3], triangle: [u32; 3]) -> Option<f32> {
    let [a, b, c] = triangle;
    let a = usize::try_from(a).ok()?;
    let b = usize::try_from(b).ok()?;
    let c = usize::try_from(c).ok()?;
    if a >= positions.len() || b >= positions.len() || c >= positions.len() {
        return None;
    }
    let area = (positions[b] - positions[a])
        .cross(positions[c] - positions[a])
        .length()
        * 0.5;
    if area.is_finite() && area > 1.0e-9 {
        Some(area)
    } else {
        None
    }
}

pub(crate) fn mesh_surface_index_hash(indices: &[[u32; 3]]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for triangle in indices {
        for index in triangle {
            for byte in index.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    hash
}
