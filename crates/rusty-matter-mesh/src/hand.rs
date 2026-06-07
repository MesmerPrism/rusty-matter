use rusty_matter_model::Vec3;

use crate::{MatterMeshError, MeshSurfaceTopologyKey, TriangleMeshSurface};

/// Schema ID for hand rig capture payloads.
pub const HAND_RIG_CAPTURE_SCHEMA_ID: &str = "rusty.matter.hand.rig_capture.v1";
/// Schema ID for hand joint frame payloads.
pub const HAND_JOINT_FRAME_SCHEMA_ID: &str = "rusty.matter.hand.joint_frame.v1";
/// Schema ID for hand validation mesh frames.
pub const HAND_VALIDATION_MESH_FRAME_SCHEMA_ID: &str = "rusty.matter.hand.validation_mesh_frame.v1";

/// Hand side for hand-specific mesh payloads.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Handedness {
    /// Hand side is unknown or not applicable.
    Unknown,
    /// Left hand.
    Left,
    /// Right hand.
    Right,
}

impl Default for Handedness {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Hand rig capture metadata around a bind-pose triangle mesh.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HandRigCapture {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable capture identifier.
    pub rig_capture_id: String,
    /// Captured hand side.
    pub handedness: Handedness,
    /// Reference-space label assigned by the provider.
    pub reference_space: String,
    /// Provider/source label.
    pub source: String,
    /// Bind-pose mesh surface.
    pub bind_surface: TriangleMeshSurface,
    /// Optional bind-pose vertex normals.
    pub bind_normals: Vec<Vec3>,
    /// Bind-surface topology key.
    pub topology_key: MeshSurfaceTopologyKey,
    /// Parent index per joint, with negative values indicating no parent.
    pub joint_parent_indices: Vec<i16>,
    /// Optional joint radii in meters.
    pub joint_radii_m: Vec<f32>,
    /// Up to four influencing joint indices per vertex.
    pub vertex_joint_indices: Vec<[u16; 4]>,
    /// Up to four skinning weights per vertex.
    pub vertex_joint_weights: Vec<[f32; 4]>,
}

impl HandRigCapture {
    /// Builds a minimal rig capture around a bind surface.
    #[must_use]
    pub fn from_bind_surface(
        rig_capture_id: impl Into<String>,
        handedness: Handedness,
        reference_space: impl Into<String>,
        source: impl Into<String>,
        bind_surface: TriangleMeshSurface,
    ) -> Self {
        let topology_key = bind_surface.topology_key();
        Self {
            schema_id: HAND_RIG_CAPTURE_SCHEMA_ID.to_owned(),
            rig_capture_id: rig_capture_id.into(),
            handedness,
            reference_space: reference_space.into(),
            source: source.into(),
            bind_surface,
            bind_normals: Vec::new(),
            topology_key,
            joint_parent_indices: Vec::new(),
            joint_radii_m: Vec::new(),
            vertex_joint_indices: Vec::new(),
            vertex_joint_weights: Vec::new(),
        }
    }

    /// Validates rig capture shape and topology identity.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the payload is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != HAND_RIG_CAPTURE_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: HAND_RIG_CAPTURE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.rig_capture_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyHandRigCaptureId);
        }
        validate_source_labels(&self.reference_space, &self.source)?;
        self.bind_surface.validate()?;
        if self.topology_key != self.bind_surface.topology_key() {
            return Err(MatterMeshError::InvalidHandPayload(
                "rig topology key must match bind surface",
            ));
        }
        validate_optional_normals(&self.bind_normals, self.bind_surface.vertex_count())?;
        validate_joint_metadata(&self.joint_parent_indices, &self.joint_radii_m)?;
        validate_skinning_metadata(
            &self.vertex_joint_indices,
            &self.vertex_joint_weights,
            self.bind_surface.vertex_count(),
        )?;
        Ok(())
    }

    /// Returns the bind surface.
    #[must_use]
    pub fn bind_surface(&self) -> &TriangleMeshSurface {
        &self.bind_surface
    }
}

/// One hand joint pose in a provider reference space.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HandJointPose {
    /// Joint position.
    pub position: Vec3,
    /// Joint orientation as `[x, y, z, w]`.
    pub orientation_xyzw: [f32; 4],
    /// Optional provider radius in meters.
    pub radius_m: f32,
}

impl HandJointPose {
    /// Returns whether the pose is finite.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.position.is_finite()
            && self.orientation_xyzw.iter().all(|value| value.is_finite())
            && self.radius_m.is_finite()
            && self.radius_m >= 0.0
    }
}

/// One recorded hand joint-motion frame.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HandJointFrame {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable frame identifier.
    pub frame_id: String,
    /// Captured hand side.
    pub handedness: Handedness,
    /// Reference-space label assigned by the provider.
    pub reference_space: String,
    /// Provider/source label.
    pub source: String,
    /// Frame time in seconds.
    pub time_seconds: f32,
    /// Joint poses.
    pub poses: Vec<HandJointPose>,
    /// Optional confidence value per joint.
    pub confidence: Vec<f32>,
}

impl HandJointFrame {
    /// Validates joint frame shape.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the payload is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != HAND_JOINT_FRAME_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: HAND_JOINT_FRAME_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.frame_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyHandJointFrameId);
        }
        validate_source_labels(&self.reference_space, &self.source)?;
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(MatterMeshError::InvalidHandPayload(
                "joint frame time_seconds must be finite and non-negative",
            ));
        }
        if self.poses.is_empty() || !self.poses.iter().all(HandJointPose::is_valid) {
            return Err(MatterMeshError::InvalidHandPayload(
                "joint frame poses must be finite and non-empty",
            ));
        }
        if !self.confidence.is_empty()
            && (self.confidence.len() != self.poses.len()
                || !self
                    .confidence
                    .iter()
                    .all(|value| value.is_finite() && *value >= 0.0 && *value <= 1.0))
        {
            return Err(MatterMeshError::InvalidHandPayload(
                "joint frame confidence must match poses and stay in [0, 1]",
            ));
        }
        Ok(())
    }
}

/// Deformed hand validation mesh frame.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HandValidationMeshFrame {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable frame identifier.
    pub frame_id: String,
    /// Captured hand side.
    pub handedness: Handedness,
    /// Reference-space label assigned by the provider.
    pub reference_space: String,
    /// Provider/source label.
    pub source: String,
    /// Frame time in seconds.
    pub time_seconds: f32,
    /// Deformed mesh surface.
    pub surface: TriangleMeshSurface,
    /// Optional deformed vertex normals.
    pub normals: Vec<Vec3>,
    /// Deformed-surface topology key.
    pub topology_key: MeshSurfaceTopologyKey,
}

impl HandValidationMeshFrame {
    /// Wraps a mesh surface as a hand validation frame.
    #[must_use]
    pub fn from_surface(
        frame_id: impl Into<String>,
        handedness: Handedness,
        reference_space: impl Into<String>,
        source: impl Into<String>,
        time_seconds: f32,
        surface: TriangleMeshSurface,
    ) -> Self {
        let topology_key = surface.topology_key();
        Self {
            schema_id: HAND_VALIDATION_MESH_FRAME_SCHEMA_ID.to_owned(),
            frame_id: frame_id.into(),
            handedness,
            reference_space: reference_space.into(),
            source: source.into(),
            time_seconds,
            surface,
            normals: Vec::new(),
            topology_key,
        }
    }

    /// Validates frame shape and topology identity.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the payload is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != HAND_VALIDATION_MESH_FRAME_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: HAND_VALIDATION_MESH_FRAME_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.frame_id.trim().is_empty() {
            return Err(MatterMeshError::EmptyHandFrameId);
        }
        validate_source_labels(&self.reference_space, &self.source)?;
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(MatterMeshError::InvalidHandPayload(
                "hand mesh frame time_seconds must be finite and non-negative",
            ));
        }
        self.surface.validate()?;
        if self.topology_key != self.surface.topology_key() {
            return Err(MatterMeshError::InvalidHandPayload(
                "hand mesh frame topology key must match surface",
            ));
        }
        validate_optional_normals(&self.normals, self.surface.vertex_count())?;
        Ok(())
    }

    /// Returns the shared mesh surface consumed by SDF, collider, and sampling.
    #[must_use]
    pub fn surface(&self) -> &TriangleMeshSurface {
        &self.surface
    }
}

fn validate_source_labels(reference_space: &str, source: &str) -> Result<(), MatterMeshError> {
    if reference_space.trim().is_empty() || source.trim().is_empty() {
        return Err(MatterMeshError::InvalidHandPayload(
            "reference_space and source must be non-empty",
        ));
    }
    Ok(())
}

fn validate_optional_normals(normals: &[Vec3], vertex_count: usize) -> Result<(), MatterMeshError> {
    if !normals.is_empty()
        && (normals.len() != vertex_count || !normals.iter().copied().all(Vec3::is_finite))
    {
        return Err(MatterMeshError::InvalidHandPayload(
            "normals must be empty or match vertex count with finite vectors",
        ));
    }
    Ok(())
}

fn validate_joint_metadata(parents: &[i16], radii: &[f32]) -> Result<(), MatterMeshError> {
    if !radii.is_empty() && radii.len() != parents.len() {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint radii must match joint parent count",
        ));
    }
    if !radii
        .iter()
        .all(|radius| radius.is_finite() && *radius >= 0.0)
    {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint radii must be finite and non-negative",
        ));
    }
    for (index, parent) in parents.iter().copied().enumerate() {
        if parent >= 0 && usize::try_from(parent).map_or(true, |parent| parent >= index) {
            return Err(MatterMeshError::InvalidHandPayload(
                "joint parent indices must refer to earlier joints",
            ));
        }
    }
    Ok(())
}

fn validate_skinning_metadata(
    joint_indices: &[[u16; 4]],
    joint_weights: &[[f32; 4]],
    vertex_count: usize,
) -> Result<(), MatterMeshError> {
    if joint_indices.is_empty() && joint_weights.is_empty() {
        return Ok(());
    }
    if joint_indices.len() != vertex_count || joint_weights.len() != vertex_count {
        return Err(MatterMeshError::InvalidHandPayload(
            "skinning metadata must match vertex count",
        ));
    }
    for weights in joint_weights {
        let sum: f32 = weights.iter().sum();
        if !weights
            .iter()
            .all(|weight| weight.is_finite() && *weight >= 0.0)
            || sum > 1.000_1
        {
            return Err(MatterMeshError::InvalidHandPayload(
                "skinning weights must be finite, non-negative, and sum to at most one",
            ));
        }
    }
    Ok(())
}
