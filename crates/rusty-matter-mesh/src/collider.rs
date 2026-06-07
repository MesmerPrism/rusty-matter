use rusty_matter_model::Vec3;

use crate::math::normalize_or;
use crate::{
    MatterMeshError, MeshSurfaceTopologyKey, SurfaceDistanceSampler, SurfaceDistanceSamplerConfig,
    TriangleMeshSurface,
};

/// Schema ID for dynamic mesh collider configuration.
pub const DYNAMIC_MESH_COLLIDER_CONFIG_SCHEMA_ID: &str =
    "rusty.matter.mesh.dynamic_collider_config.v1";
/// Schema ID for dynamic mesh collider update summaries.
pub const DYNAMIC_MESH_COLLIDER_UPDATE_SCHEMA_ID: &str =
    "rusty.matter.mesh.dynamic_collider_update.v1";
/// Schema ID for dynamic mesh collider diagnostic shells.
pub const DYNAMIC_MESH_COLLIDER_SHELL_SCHEMA_ID: &str =
    "rusty.matter.mesh.dynamic_collider_shell.v1";
/// Schema ID for dynamic mesh collider contacts.
pub const DYNAMIC_MESH_COLLIDER_CONTACT_SCHEMA_ID: &str =
    "rusty.matter.mesh.dynamic_collider_contact.v1";

/// Dynamic mesh collider configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicMeshColliderConfig {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable config identifier.
    pub collider_config_id: String,
    /// Whether collider payload generation is enabled.
    pub enabled: bool,
    /// Surface inflation distance.
    pub surface_inflation: f32,
    /// Contact padding used by overlap queries.
    pub contact_padding: f32,
    /// Whether downstream adapters should prefer convex collider import.
    pub prefer_convex: bool,
    /// Maximum triangle count considered convex-import eligible.
    pub max_convex_triangle_count: usize,
    /// Whether to emit a diagnostic shell.
    pub diagnostic_shell_enabled: bool,
    /// Additional diagnostic shell inflation.
    pub diagnostic_shell_inflation: f32,
}

impl Default for DynamicMeshColliderConfig {
    fn default() -> Self {
        Self {
            schema_id: DYNAMIC_MESH_COLLIDER_CONFIG_SCHEMA_ID.to_owned(),
            collider_config_id: "mesh.dynamic_collider.default".to_owned(),
            enabled: true,
            surface_inflation: 0.0,
            contact_padding: 0.0,
            prefer_convex: false,
            max_convex_triangle_count: 256,
            diagnostic_shell_enabled: true,
            diagnostic_shell_inflation: 0.0,
        }
    }
}

impl DynamicMeshColliderConfig {
    /// Validates collider config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the config is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != DYNAMIC_MESH_COLLIDER_CONFIG_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: DYNAMIC_MESH_COLLIDER_CONFIG_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.collider_config_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyColliderConfigId);
        }
        if !self.surface_inflation.is_finite() || self.surface_inflation < 0.0 {
            return Err(MatterMeshError::InvalidColliderConfig(
                "surface_inflation must be finite and non-negative",
            ));
        }
        if !self.contact_padding.is_finite() || self.contact_padding < 0.0 {
            return Err(MatterMeshError::InvalidColliderConfig(
                "contact_padding must be finite and non-negative",
            ));
        }
        if !self.diagnostic_shell_inflation.is_finite() || self.diagnostic_shell_inflation < 0.0 {
            return Err(MatterMeshError::InvalidColliderConfig(
                "diagnostic_shell_inflation must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// Status from one dynamic mesh collider update.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicMeshColliderUpdateStatus {
    /// Collider generation is disabled.
    Disabled,
    /// Collider payloads were initialized.
    Initialized,
    /// Collider payloads were updated with matching topology.
    Updated,
    /// Collider payloads were rebuilt after topology changed.
    ChangedTopology,
    /// Surface or config was invalid.
    InvalidSurface,
}

/// Summary from one dynamic collider update.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicMeshColliderUpdate {
    /// Schema identifier.
    pub schema_id: String,
    /// Update status.
    pub status: DynamicMeshColliderUpdateStatus,
    /// Current topology key, if available.
    pub topology_key: Option<MeshSurfaceTopologyKey>,
    /// Collider surface vertex count.
    pub vertex_count: usize,
    /// Collider surface triangle count.
    pub triangle_count: usize,
    /// Requested convex import preference.
    pub convex_preferred: bool,
    /// Whether current geometry is below the convex triangle budget.
    pub convex_eligible: bool,
    /// Diagnostic shell vertex count.
    pub diagnostic_shell_vertex_count: usize,
    /// Diagnostic shell triangle count.
    pub diagnostic_shell_triangle_count: usize,
}

/// Diagnostic collider shell payload.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicMeshColliderShell {
    /// Schema identifier.
    pub schema_id: String,
    /// Inflated diagnostic shell surface.
    pub surface: TriangleMeshSurface,
    /// Additional shell inflation over collider surface inflation.
    pub shell_inflation: f32,
}

/// Closest-point contact against a generated collider surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicMeshColliderContact {
    /// Schema identifier.
    pub schema_id: String,
    /// Closest point on the mesh.
    pub point: Vec3,
    /// Triangle normal at the contact.
    pub normal: Vec3,
    /// Euclidean distance from query point to closest point.
    pub distance: f32,
    /// Source triangle index.
    pub triangle_index: usize,
}

/// Framework-neutral dynamic mesh collider payload builder.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicMeshCollider {
    config: DynamicMeshColliderConfig,
    collider_surface: Option<TriangleMeshSurface>,
    distance_sampler: Option<SurfaceDistanceSampler>,
    diagnostic_shell: Option<DynamicMeshColliderShell>,
    topology_key: Option<MeshSurfaceTopologyKey>,
}

impl DynamicMeshCollider {
    /// Creates a collider.
    #[must_use]
    pub fn new(config: DynamicMeshColliderConfig) -> Self {
        Self {
            config,
            collider_surface: None,
            distance_sampler: None,
            diagnostic_shell: None,
            topology_key: None,
        }
    }

    /// Returns the config.
    #[must_use]
    pub fn config(&self) -> &DynamicMeshColliderConfig {
        &self.config
    }

    /// Returns the collider surface.
    #[must_use]
    pub fn collider_surface(&self) -> Option<&TriangleMeshSurface> {
        self.collider_surface.as_ref()
    }

    /// Returns the accelerated distance sampler for the collider surface.
    #[must_use]
    pub fn distance_sampler(&self) -> Option<&SurfaceDistanceSampler> {
        self.distance_sampler.as_ref()
    }

    /// Returns the diagnostic shell.
    #[must_use]
    pub fn diagnostic_shell(&self) -> Option<&DynamicMeshColliderShell> {
        self.diagnostic_shell.as_ref()
    }

    /// Clears generated surfaces.
    pub fn clear(&mut self) {
        self.collider_surface = None;
        self.distance_sampler = None;
        self.diagnostic_shell = None;
        self.topology_key = None;
    }

    /// Updates collider payloads from a surface.
    #[must_use]
    pub fn update_from_surface(
        &mut self,
        surface: &TriangleMeshSurface,
    ) -> DynamicMeshColliderUpdate {
        if self.config.validate().is_err() {
            self.clear();
            return self.update_summary(DynamicMeshColliderUpdateStatus::InvalidSurface);
        }
        if !self.config.enabled {
            self.clear();
            return self.update_summary(DynamicMeshColliderUpdateStatus::Disabled);
        }
        if surface.validate().is_err() {
            self.clear();
            return self.update_summary(DynamicMeshColliderUpdateStatus::InvalidSurface);
        }

        let next_key = surface.topology_key();
        let Ok(collider_surface) =
            build_dynamic_mesh_collider_surface(surface, self.config.surface_inflation)
        else {
            self.clear();
            return self.update_summary(DynamicMeshColliderUpdateStatus::InvalidSurface);
        };
        let Ok(distance_sampler) =
            collider_surface.distance_sampler(SurfaceDistanceSamplerConfig::default())
        else {
            self.clear();
            return self.update_summary(DynamicMeshColliderUpdateStatus::InvalidSurface);
        };
        let diagnostic_shell = if self.config.diagnostic_shell_enabled {
            let shell_inflation =
                self.config.surface_inflation + self.config.diagnostic_shell_inflation;
            build_dynamic_mesh_collider_surface(surface, shell_inflation)
                .ok()
                .map(|surface| DynamicMeshColliderShell {
                    schema_id: DYNAMIC_MESH_COLLIDER_SHELL_SCHEMA_ID.to_owned(),
                    surface,
                    shell_inflation: self.config.diagnostic_shell_inflation,
                })
        } else {
            None
        };

        let status = if self.topology_key.is_none() {
            DynamicMeshColliderUpdateStatus::Initialized
        } else if self.topology_key.as_ref() == Some(&next_key) {
            DynamicMeshColliderUpdateStatus::Updated
        } else {
            DynamicMeshColliderUpdateStatus::ChangedTopology
        };

        self.collider_surface = Some(collider_surface);
        self.distance_sampler = Some(distance_sampler);
        self.diagnostic_shell = diagnostic_shell;
        self.topology_key = Some(next_key);
        self.update_summary(status)
    }

    /// Returns the closest point on the current collider surface.
    #[must_use]
    pub fn closest_point(&self, point: Vec3) -> Option<DynamicMeshColliderContact> {
        let sample = self.distance_sampler.as_ref()?.sample(point)?;
        Some(DynamicMeshColliderContact {
            schema_id: DYNAMIC_MESH_COLLIDER_CONTACT_SCHEMA_ID.to_owned(),
            point: sample.point,
            normal: sample.normal,
            distance: sample.distance,
            triangle_index: sample.triangle_index,
        })
    }

    /// Returns whether a sphere overlaps the current collider surface.
    #[must_use]
    pub fn overlaps_sphere(&self, center: Vec3, radius: f32) -> bool {
        let Some(contact) = self.closest_point(center) else {
            return false;
        };
        let radius = radius.max(0.0) + self.config.contact_padding.max(0.0);
        contact.distance <= radius
    }

    fn update_summary(&self, status: DynamicMeshColliderUpdateStatus) -> DynamicMeshColliderUpdate {
        let shell = self.diagnostic_shell.as_ref().map(|shell| &shell.surface);
        let surface = self.collider_surface.as_ref();
        let triangle_count = surface.map_or(0, TriangleMeshSurface::triangle_count);
        DynamicMeshColliderUpdate {
            schema_id: DYNAMIC_MESH_COLLIDER_UPDATE_SCHEMA_ID.to_owned(),
            status,
            topology_key: self.topology_key.clone(),
            vertex_count: surface.map_or(0, TriangleMeshSurface::vertex_count),
            triangle_count,
            convex_preferred: self.config.prefer_convex,
            convex_eligible: self.config.prefer_convex
                && triangle_count <= self.config.max_convex_triangle_count,
            diagnostic_shell_vertex_count: shell.map_or(0, TriangleMeshSurface::vertex_count),
            diagnostic_shell_triangle_count: shell.map_or(0, TriangleMeshSurface::triangle_count),
        }
    }
}

impl Default for DynamicMeshCollider {
    fn default() -> Self {
        Self::new(DynamicMeshColliderConfig::default())
    }
}

/// Builds collider geometry from a surface and inflation distance.
///
/// # Errors
///
/// Returns [`MatterMeshError`] when the surface or inflation is invalid.
pub fn build_dynamic_mesh_collider_surface(
    surface: &TriangleMeshSurface,
    surface_inflation: f32,
) -> Result<TriangleMeshSurface, MatterMeshError> {
    surface.validate()?;
    if !surface_inflation.is_finite() || surface_inflation < 0.0 {
        return Err(MatterMeshError::InvalidColliderConfig(
            "surface_inflation must be finite and non-negative",
        ));
    }
    if surface_inflation <= 0.0 {
        return Ok(surface.clone());
    }

    let normals = surface_vertex_normals(surface);
    let center = surface_center(surface);
    let positions = surface
        .positions
        .iter()
        .copied()
        .enumerate()
        .map(|(index, position)| {
            let normal = normals
                .get(index)
                .copied()
                .filter(|normal| normal.length_squared() > 1.0e-10)
                .unwrap_or_else(|| normalize_or(position - center, Vec3::new(0.0, 1.0, 0.0)));
            position + normal * surface_inflation
        })
        .collect::<Vec<_>>();
    Ok(TriangleMeshSurface::new(
        format!("{}.collider", surface.surface_id),
        positions,
        surface.triangles.clone(),
    ))
}

fn surface_vertex_normals(surface: &TriangleMeshSurface) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; surface.positions.len()];
    for triangle in &surface.triangles {
        let [a, b, c] = *triangle;
        let Ok(a) = usize::try_from(a) else {
            continue;
        };
        let Ok(b) = usize::try_from(b) else {
            continue;
        };
        let Ok(c) = usize::try_from(c) else {
            continue;
        };
        if a >= surface.positions.len()
            || b >= surface.positions.len()
            || c >= surface.positions.len()
        {
            continue;
        }
        let normal = (surface.positions[b] - surface.positions[a])
            .cross(surface.positions[c] - surface.positions[a]);
        if !normal.is_finite() || normal.length_squared() <= 1.0e-14 {
            continue;
        }
        normals[a] = normals[a] + normal;
        normals[b] = normals[b] + normal;
        normals[c] = normals[c] + normal;
    }
    normals
        .into_iter()
        .map(|normal| normalize_or(normal, Vec3::ZERO))
        .collect()
}

fn surface_center(surface: &TriangleMeshSurface) -> Vec3 {
    if surface.positions.is_empty() {
        return Vec3::ZERO;
    }
    let mut sum = Vec3::ZERO;
    for position in &surface.positions {
        sum = sum + *position;
    }
    sum / surface.positions.len() as f32
}
