use rusty_matter_model::Vec3;

use crate::math::normalize_or;
use crate::{
    sample_mesh_surface_points, MatterMeshError, MeshSurfaceSample, MeshSurfaceSampleConfig,
    MeshSurfaceSampleSet, MeshSurfaceTopologyKey, TriangleMeshSurface,
};

/// Schema ID for mesh coordinate maps.
pub const MESH_COORDINATE_MAP_SCHEMA_ID: &str = "rusty.matter.mesh.coordinate_map.v1";
/// Schema ID for mesh coordinate frame configuration.
pub const MESH_COORDINATE_FRAME_CONFIG_SCHEMA_ID: &str =
    "rusty.matter.mesh.coordinate_frame_config.v1";
/// Schema ID for mesh coordinate local frames.
pub const MESH_COORDINATE_LOCAL_FRAME_SCHEMA_ID: &str =
    "rusty.matter.mesh.coordinate_local_frame.v1";
/// Schema ID for mesh coordinate frame sets.
pub const MESH_COORDINATE_FRAME_SET_SCHEMA_ID: &str = "rusty.matter.mesh.coordinate_frame_set.v1";

/// Clamp mode for local coordinate-frame displacements.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshLocalDisplacementClampMode {
    /// Clamp each normalized component independently to `[-1, 1]`.
    PerAxis,
    /// Clamp the normalized displacement vector into a unit ellipsoid.
    Ellipsoid,
}

impl Default for MeshLocalDisplacementClampMode {
    fn default() -> Self {
        Self::Ellipsoid
    }
}

/// Configuration for local coordinate frames derived from mesh samples.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCoordinateFrameConfig {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable frame config identifier.
    pub frame_config_id: String,
    /// Maximum X/Y/Z displacement in local frame units.
    pub max_displacement: Vec3,
    /// Normalized displacement clamp mode.
    pub clamp_mode: MeshLocalDisplacementClampMode,
}

impl Default for MeshCoordinateFrameConfig {
    fn default() -> Self {
        Self {
            schema_id: MESH_COORDINATE_FRAME_CONFIG_SCHEMA_ID.to_owned(),
            frame_config_id: "mesh.coordinate_frame.default".to_owned(),
            max_displacement: Vec3::new(0.05, 0.05, 0.05),
            clamp_mode: MeshLocalDisplacementClampMode::Ellipsoid,
        }
    }
}

impl MeshCoordinateFrameConfig {
    /// Validates frame configuration.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the config is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != MESH_COORDINATE_FRAME_CONFIG_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: MESH_COORDINATE_FRAME_CONFIG_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.frame_config_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyCoordinateFrameConfigId);
        }
        if !self.max_displacement.is_finite()
            || self.max_displacement.x < 0.0
            || self.max_displacement.y < 0.0
            || self.max_displacement.z < 0.0
        {
            return Err(MatterMeshError::InvalidCoordinateFrameConfig(
                "max_displacement must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// A stable local frame anchored to one sampled surface coordinate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCoordinateLocalFrame {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable local frame identifier.
    pub frame_id: String,
    /// Surface anchor position.
    pub anchor: Vec3,
    /// Local X tangent axis.
    pub axis_x: Vec3,
    /// Local Y tangent axis.
    pub axis_y: Vec3,
    /// Local Z normal axis.
    pub axis_z: Vec3,
    /// Maximum X/Y/Z displacement in this frame.
    pub max_displacement: Vec3,
}

impl MeshCoordinateLocalFrame {
    /// Builds a local frame from one mesh sample and frame config.
    #[must_use]
    pub fn from_sample(
        frame_id: impl Into<String>,
        sample: &MeshSurfaceSample,
        config: &MeshCoordinateFrameConfig,
    ) -> Self {
        let (axis_x, axis_y, axis_z) = coordinate_axes_from_normal(sample.normal);
        Self {
            schema_id: MESH_COORDINATE_LOCAL_FRAME_SCHEMA_ID.to_owned(),
            frame_id: frame_id.into(),
            anchor: sample.position,
            axis_x,
            axis_y,
            axis_z,
            max_displacement: config.max_displacement,
        }
    }

    /// Returns a displaced point for a normalized local driver.
    #[must_use]
    pub fn displace(
        &self,
        normalized_driver: Vec3,
        clamp_mode: MeshLocalDisplacementClampMode,
    ) -> Vec3 {
        let driver = clamp_normalized_driver(normalized_driver, clamp_mode);
        self.anchor
            + self.axis_x * (driver.x * self.max_displacement.x)
            + self.axis_y * (driver.y * self.max_displacement.y)
            + self.axis_z * (driver.z * self.max_displacement.z)
    }

    /// Returns whether this frame is internally consistent.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.schema_id == MESH_COORDINATE_LOCAL_FRAME_SCHEMA_ID
            && !self.frame_id.trim().is_empty()
            && self.anchor.is_finite()
            && unit_axis_is_valid(self.axis_x)
            && unit_axis_is_valid(self.axis_y)
            && unit_axis_is_valid(self.axis_z)
            && self.max_displacement.is_finite()
            && self.max_displacement.x >= 0.0
            && self.max_displacement.y >= 0.0
            && self.max_displacement.z >= 0.0
            && self.axis_x.dot(self.axis_y).abs() <= 1.0e-3
            && self.axis_y.dot(self.axis_z).abs() <= 1.0e-3
            && self.axis_z.dot(self.axis_x).abs() <= 1.0e-3
    }
}

/// Local coordinate frames derived from a mesh sample set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCoordinateFrameSet {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable frame-set identifier.
    pub frame_set_id: String,
    /// Source topology key.
    pub topology_key: MeshSurfaceTopologyKey,
    /// Local frames, one per mesh surface sample.
    pub frames: Vec<MeshCoordinateLocalFrame>,
    /// Clamp mode used by consumers when applying normalized drivers.
    pub clamp_mode: MeshLocalDisplacementClampMode,
}

impl MeshCoordinateFrameSet {
    /// Builds local frames from a mesh sample set.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when samples or config are invalid.
    pub fn from_samples(
        frame_set_id: impl Into<String>,
        samples: &MeshSurfaceSampleSet,
        config: &MeshCoordinateFrameConfig,
    ) -> Result<Self, MatterMeshError> {
        config.validate()?;
        if !samples.is_valid() {
            return Err(MatterMeshError::InvalidCoordinateMap(
                "sample set must be valid",
            ));
        }
        let frame_set_id = frame_set_id.into();
        if frame_set_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyCoordinateFrameSetId);
        }
        let frames = samples
            .samples
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                MeshCoordinateLocalFrame::from_sample(
                    format!("{frame_set_id}.frame.{index:04}"),
                    sample,
                    config,
                )
            })
            .collect::<Vec<_>>();
        let frame_set = Self {
            schema_id: MESH_COORDINATE_FRAME_SET_SCHEMA_ID.to_owned(),
            frame_set_id,
            topology_key: samples.topology_key.clone(),
            frames,
            clamp_mode: config.clamp_mode,
        };
        if frame_set.is_valid(samples.len()) {
            Ok(frame_set)
        } else {
            Err(MatterMeshError::InvalidCoordinateMap(
                "generated coordinate frames did not validate",
            ))
        }
    }

    /// Returns whether this frame set matches the expected sample count.
    #[must_use]
    pub fn is_valid(&self, sample_count: usize) -> bool {
        self.schema_id == MESH_COORDINATE_FRAME_SET_SCHEMA_ID
            && !self.frame_set_id.trim().is_empty()
            && self.frames.len() == sample_count
            && self.frames.iter().all(MeshCoordinateLocalFrame::is_valid)
    }
}

/// Stable coordinate map for a dynamic triangle mesh topology.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCoordinateMap {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable coordinate-map identifier.
    pub coordinate_map_id: String,
    /// Source topology key.
    pub topology_key: MeshSurfaceTopologyKey,
    /// Sampling configuration used to build coordinates.
    pub sample_config: MeshSurfaceSampleConfig,
    /// Surface samples with barycentric anchors and neighbor tiers.
    pub samples: MeshSurfaceSampleSet,
    /// Local-frame configuration used to build frames.
    pub frame_config: MeshCoordinateFrameConfig,
    /// Local frames, one per sample.
    pub frames: MeshCoordinateFrameSet,
}

impl MeshCoordinateMap {
    /// Builds a coordinate map from a mesh surface.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the surface, sample config, or frame
    /// config is invalid.
    pub fn from_surface(
        coordinate_map_id: impl Into<String>,
        surface: &TriangleMeshSurface,
        sample_config: MeshSurfaceSampleConfig,
        frame_config: MeshCoordinateFrameConfig,
    ) -> Result<Self, MatterMeshError> {
        let coordinate_map_id = coordinate_map_id.into();
        if coordinate_map_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyCoordinateMapId);
        }
        let samples = sample_mesh_surface_points(surface, &sample_config)?;
        let frames = MeshCoordinateFrameSet::from_samples(
            format!("{coordinate_map_id}.frames"),
            &samples,
            &frame_config,
        )?;
        let map = Self {
            schema_id: MESH_COORDINATE_MAP_SCHEMA_ID.to_owned(),
            coordinate_map_id,
            topology_key: surface.topology_key(),
            sample_config,
            samples,
            frame_config,
            frames,
        };
        if map.is_valid_for_surface(surface) {
            Ok(map)
        } else {
            Err(MatterMeshError::InvalidCoordinateMap(
                "generated coordinate map did not validate",
            ))
        }
    }

    /// Returns whether this map can be reused for the current surface.
    #[must_use]
    pub fn is_valid_for_surface(&self, surface: &TriangleMeshSurface) -> bool {
        surface.validate().is_ok()
            && self.schema_id == MESH_COORDINATE_MAP_SCHEMA_ID
            && !self.coordinate_map_id.trim().is_empty()
            && self.topology_key == surface.topology_key()
            && self.samples.topology_key == self.topology_key
            && self.samples.surface_id == surface.surface_id
            && self.samples.is_valid()
            && self.frame_config.validate().is_ok()
            && self.frames.topology_key == self.topology_key
            && self.frames.is_valid(self.samples.len())
    }
}

fn coordinate_axes_from_normal(normal: Vec3) -> (Vec3, Vec3, Vec3) {
    let up = Vec3::new(0.0, 1.0, 0.0);
    let right = Vec3::new(1.0, 0.0, 0.0);
    let axis_z = normalize_or(normal, up);
    let helper = if axis_z.dot(up).abs() < 0.92 {
        up
    } else {
        right
    };
    let axis_x = normalize_or(helper.cross(axis_z), right);
    let axis_y = normalize_or(axis_z.cross(axis_x), up);
    (axis_x, axis_y, axis_z)
}

fn clamp_normalized_driver(driver: Vec3, clamp_mode: MeshLocalDisplacementClampMode) -> Vec3 {
    let mut clamped = Vec3::new(
        finite_or_zero(driver.x).clamp(-1.0, 1.0),
        finite_or_zero(driver.y).clamp(-1.0, 1.0),
        finite_or_zero(driver.z).clamp(-1.0, 1.0),
    );
    if clamp_mode == MeshLocalDisplacementClampMode::Ellipsoid {
        let length = clamped.length();
        if length > 1.0 {
            clamped = clamped / length;
        }
    }
    clamped
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn unit_axis_is_valid(axis: Vec3) -> bool {
    axis.is_finite() && (axis.length() - 1.0).abs() <= 1.0e-3
}
