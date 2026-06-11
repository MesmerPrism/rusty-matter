use rusty_matter_model::Vec3;

use crate::{MatterMeshError, MeshSurfaceTopologyKey, TriangleMeshSurface};

/// Schema ID for hand rig capture payloads.
pub const HAND_RIG_CAPTURE_SCHEMA_ID: &str = "rusty.matter.hand.rig_capture.v1";
/// Schema ID for hand joint frame payloads.
pub const HAND_JOINT_FRAME_SCHEMA_ID: &str = "rusty.matter.hand.joint_frame.v1";
/// Schema ID for hand validation mesh frames.
pub const HAND_VALIDATION_MESH_FRAME_SCHEMA_ID: &str = "rusty.matter.hand.validation_mesh_frame.v1";
/// Maximum weighted joint influences used by Matter hand skinning.
pub const HAND_SKINNING_MATRIX_INFLUENCE_COUNT: usize = 4;

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

/// One bounded joint-matrix skinning oracle sample.
///
/// This is a compact diagnostic/oracle payload for GPU adapters. It carries
/// one bind vertex, up to four weighted bind-pose-to-frame joint matrices, and
/// the Matter CPU-skinned expected position for that vertex.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HandSkinningMatrixSample {
    /// Source bind-mesh vertex index.
    pub vertex_index: usize,
    /// Bind-pose vertex position as `[x, y, z, 1]`.
    pub bind_position: [f32; 4],
    /// Influencing joint indices in the original rig.
    pub joint_indices: [u16; HAND_SKINNING_MATRIX_INFLUENCE_COUNT],
    /// Skinning weights in the same order as `joint_indices`.
    pub joint_weights: [f32; HAND_SKINNING_MATRIX_INFLUENCE_COUNT],
    /// Row-major matrices mapping bind positions to current joint-frame space.
    pub joint_matrices: [[[f32; 4]; 4]; HAND_SKINNING_MATRIX_INFLUENCE_COUNT],
    /// Matter CPU-skinned oracle position as `[x, y, z, 1]`.
    pub expected_position: [f32; 4],
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
    /// Bind-pose joint transforms in the same reference space as bind vertices.
    pub joint_bind_poses: Vec<HandJointPose>,
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
            joint_bind_poses: Vec::new(),
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
        validate_joint_metadata(
            &self.joint_parent_indices,
            &self.joint_radii_m,
            &self.joint_bind_poses,
        )?;
        validate_skinning_metadata(
            &self.vertex_joint_indices,
            &self.vertex_joint_weights,
            self.bind_surface.vertex_count(),
            self.joint_count(),
        )?;
        Ok(())
    }

    /// Returns the bind surface.
    #[must_use]
    pub fn bind_surface(&self) -> &TriangleMeshSurface {
        &self.bind_surface
    }

    /// Returns the number of joints described by the rig metadata.
    #[must_use]
    pub fn joint_count(&self) -> usize {
        self.joint_parent_indices
            .len()
            .max(self.joint_radii_m.len())
            .max(self.joint_bind_poses.len())
    }

    /// CPU-skins the bind mesh through a full bind-joint pose frame.
    ///
    /// This is the Matter-owned reference path used by GPU adapters as an
    /// oracle. Compact provider packets should be expanded to bind-joint
    /// poses before calling this method.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the rig, joint frame, or skinning
    /// metadata is invalid.
    pub fn skin_to_surface(
        &self,
        frame: &HandJointFrame,
        surface_id: impl Into<String>,
    ) -> Result<TriangleMeshSurface, MatterMeshError> {
        self.validate()?;
        frame.validate()?;
        validate_rig_frame_match(self, frame)?;
        validate_full_bind_joint_frame(self, frame)?;

        let positions = self
            .bind_surface
            .positions
            .iter()
            .copied()
            .enumerate()
            .map(|(index, position)| self.skin_vertex(index, position, frame))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TriangleMeshSurface::new(
            surface_id,
            positions,
            self.bind_surface.triangles.clone(),
        ))
    }

    /// CPU-skins the bind mesh into a hand validation frame.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the rig, joint frame, or skinning
    /// metadata is invalid.
    pub fn skin_to_validation_frame(
        &self,
        frame: &HandJointFrame,
        frame_id: impl Into<String>,
    ) -> Result<HandValidationMeshFrame, MatterMeshError> {
        let frame_id = frame_id.into();
        let surface = self.skin_to_surface(frame, format!("{frame_id}.surface"))?;
        let mut validation_frame = HandValidationMeshFrame::from_surface(
            frame_id,
            frame.handedness,
            frame.reference_space.clone(),
            frame.source.clone(),
            frame.time_seconds,
            surface,
        );
        validation_frame.normals = self.skin_normals(frame)?;
        Ok(validation_frame)
    }

    /// Builds bounded joint-matrix skinning samples with Matter CPU-oracle outputs.
    ///
    /// GPU adapters can submit these compact samples to prove that a shader is
    /// applying the same bind-pose-to-frame skinning transform as Matter's CPU
    /// reference path. This method never returns the full mesh unless
    /// `max_samples` requests it.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the rig, joint frame, or skinning
    /// metadata is invalid.
    pub fn skinning_matrix_samples(
        &self,
        frame: &HandJointFrame,
        max_samples: usize,
    ) -> Result<Vec<HandSkinningMatrixSample>, MatterMeshError> {
        self.validate()?;
        frame.validate()?;
        validate_rig_frame_match(self, frame)?;
        validate_full_bind_joint_frame(self, frame)?;

        let vertex_count = self.bind_surface.vertex_count();
        let sample_count = vertex_count.min(max_samples);
        if sample_count == 0 {
            return Ok(Vec::new());
        }

        (0..sample_count)
            .map(|sample_index| {
                let vertex_index = selected_vertex_index(vertex_count, sample_count, sample_index);
                self.skinning_matrix_sample(vertex_index, frame)
            })
            .collect()
    }

    fn skin_vertex(
        &self,
        vertex_index: usize,
        bind_vertex: Vec3,
        frame: &HandJointFrame,
    ) -> Result<Vec3, MatterMeshError> {
        if self.vertex_joint_indices.is_empty() || self.vertex_joint_weights.is_empty() {
            return Ok(bind_vertex);
        }

        let blend_indices = self.vertex_joint_indices.get(vertex_index).ok_or(
            MatterMeshError::InvalidHandPayload("skinning metadata must match vertex count"),
        )?;
        let blend_weights = self.vertex_joint_weights.get(vertex_index).ok_or(
            MatterMeshError::InvalidHandPayload("skinning metadata must match vertex count"),
        )?;
        let mut out = Vec3::ZERO;
        let mut total_weight = 0.0;
        for slot in 0..4 {
            let weight = blend_weights[slot];
            if weight <= 0.0 || !weight.is_finite() {
                continue;
            }
            let joint_index = usize::from(blend_indices[slot]);
            let bind_pose = self.joint_bind_poses.get(joint_index).ok_or(
                MatterMeshError::InvalidHandPayload(
                    "skinning joint index must reference a bind pose",
                ),
            )?;
            let joint_pose =
                frame
                    .poses
                    .get(joint_index)
                    .ok_or(MatterMeshError::InvalidHandPayload(
                        "skinning joint index must reference a frame pose",
                    ))?;
            let local = inverse_transform_point(bind_pose, bind_vertex)?;
            let skinned = transform_point(joint_pose, local)?;
            out = out + skinned * weight;
            total_weight += weight;
        }
        if total_weight > 0.0 && total_weight.is_finite() {
            Ok(out / total_weight)
        } else {
            Ok(bind_vertex)
        }
    }

    fn skinning_matrix_sample(
        &self,
        vertex_index: usize,
        frame: &HandJointFrame,
    ) -> Result<HandSkinningMatrixSample, MatterMeshError> {
        let bind_vertex = *self.bind_surface.positions.get(vertex_index).ok_or(
            MatterMeshError::InvalidHandPayload("skinning sample vertex index must be in range"),
        )?;
        let expected = self.skin_vertex(vertex_index, bind_vertex, frame)?;
        let mut sample = HandSkinningMatrixSample {
            vertex_index,
            bind_position: [bind_vertex.x, bind_vertex.y, bind_vertex.z, 1.0],
            expected_position: [expected.x, expected.y, expected.z, 1.0],
            ..HandSkinningMatrixSample::default()
        };

        if self.vertex_joint_indices.is_empty() || self.vertex_joint_weights.is_empty() {
            sample.joint_weights[0] = 1.0;
            sample.joint_matrices[0] = identity_matrix4();
            return Ok(sample);
        }

        let blend_indices = self.vertex_joint_indices.get(vertex_index).ok_or(
            MatterMeshError::InvalidHandPayload("skinning metadata must match vertex count"),
        )?;
        let blend_weights = self.vertex_joint_weights.get(vertex_index).ok_or(
            MatterMeshError::InvalidHandPayload("skinning metadata must match vertex count"),
        )?;
        let mut total_weight = 0.0;
        for slot in 0..HAND_SKINNING_MATRIX_INFLUENCE_COUNT {
            let weight = blend_weights[slot];
            sample.joint_indices[slot] = blend_indices[slot];
            sample.joint_weights[slot] = weight;
            if weight <= 0.0 || !weight.is_finite() {
                continue;
            }
            let joint_index = usize::from(blend_indices[slot]);
            let bind_pose = self.joint_bind_poses.get(joint_index).ok_or(
                MatterMeshError::InvalidHandPayload(
                    "skinning joint index must reference a bind pose",
                ),
            )?;
            let joint_pose =
                frame
                    .poses
                    .get(joint_index)
                    .ok_or(MatterMeshError::InvalidHandPayload(
                        "skinning joint index must reference a frame pose",
                    ))?;
            sample.joint_matrices[slot] = joint_skinning_matrix(bind_pose, joint_pose)?;
            total_weight += weight;
        }

        if total_weight <= 0.0 || !total_weight.is_finite() {
            sample.joint_indices = [0; HAND_SKINNING_MATRIX_INFLUENCE_COUNT];
            sample.joint_weights = [1.0, 0.0, 0.0, 0.0];
            sample.joint_matrices = [[[0.0; 4]; 4]; HAND_SKINNING_MATRIX_INFLUENCE_COUNT];
            sample.joint_matrices[0] = identity_matrix4();
        }
        Ok(sample)
    }

    fn skin_normals(&self, frame: &HandJointFrame) -> Result<Vec<Vec3>, MatterMeshError> {
        if self.bind_normals.is_empty() {
            return Ok(Vec::new());
        }
        if self.vertex_joint_indices.is_empty() || self.vertex_joint_weights.is_empty() {
            return Ok(self.bind_normals.clone());
        }

        self.bind_normals
            .iter()
            .copied()
            .enumerate()
            .map(|(index, normal)| self.skin_normal(index, normal, frame))
            .collect()
    }

    fn skin_normal(
        &self,
        vertex_index: usize,
        bind_normal: Vec3,
        frame: &HandJointFrame,
    ) -> Result<Vec3, MatterMeshError> {
        let blend_indices = self.vertex_joint_indices.get(vertex_index).ok_or(
            MatterMeshError::InvalidHandPayload("skinning metadata must match vertex count"),
        )?;
        let blend_weights = self.vertex_joint_weights.get(vertex_index).ok_or(
            MatterMeshError::InvalidHandPayload("skinning metadata must match vertex count"),
        )?;
        let mut out = Vec3::ZERO;
        let mut total_weight = 0.0;
        for slot in 0..4 {
            let weight = blend_weights[slot];
            if weight <= 0.0 || !weight.is_finite() {
                continue;
            }
            let joint_index = usize::from(blend_indices[slot]);
            let bind_pose = self.joint_bind_poses.get(joint_index).ok_or(
                MatterMeshError::InvalidHandPayload(
                    "skinning joint index must reference a bind pose",
                ),
            )?;
            let joint_pose =
                frame
                    .poses
                    .get(joint_index)
                    .ok_or(MatterMeshError::InvalidHandPayload(
                        "skinning joint index must reference a frame pose",
                    ))?;
            let local = inverse_rotate_vec3(bind_pose, bind_normal)?;
            let skinned = rotate_vec3(joint_pose, local)?;
            out = out + skinned * weight;
            total_weight += weight;
        }
        if total_weight > 0.0 && total_weight.is_finite() {
            normalize_vec3(out)
        } else {
            Ok(bind_normal)
        }
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

    /// Compares this expected validation frame with an actual frame.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when either frame or the tolerance is
    /// invalid.
    pub fn compare_with(
        &self,
        actual: &Self,
        tolerance: HandValidationMeshTolerance,
    ) -> Result<HandValidationMeshComparison, MatterMeshError> {
        self.validate()?;
        actual.validate()?;
        tolerance.validate()?;

        let mut max_position_error_m = 0.0_f32;
        let mut position_mismatch_count = 0_usize;
        for (expected, actual) in self
            .surface
            .positions
            .iter()
            .copied()
            .zip(actual.surface.positions.iter().copied())
        {
            let error = expected.distance_squared(actual).sqrt();
            max_position_error_m = max_position_error_m.max(error);
            if error > tolerance.max_position_error_m {
                position_mismatch_count += 1;
            }
        }

        let mut max_normal_error = 0.0_f32;
        let mut normal_mismatch_count = 0_usize;
        for (expected, actual) in self
            .normals
            .iter()
            .copied()
            .zip(actual.normals.iter().copied())
        {
            let error = (expected - actual).length();
            max_normal_error = max_normal_error.max(error);
            if error > tolerance.max_normal_error {
                normal_mismatch_count += 1;
            }
        }

        let expected_vertex_count = self.surface.vertex_count();
        let actual_vertex_count = actual.surface.vertex_count();
        let expected_normal_count = self.normals.len();
        let actual_normal_count = actual.normals.len();
        let topology_matched = self.topology_key == actual.topology_key;
        let passed = topology_matched
            && expected_vertex_count == actual_vertex_count
            && expected_normal_count == actual_normal_count
            && position_mismatch_count == 0
            && normal_mismatch_count == 0;

        Ok(HandValidationMeshComparison {
            expected_vertex_count,
            actual_vertex_count,
            expected_normal_count,
            actual_normal_count,
            topology_matched,
            position_mismatch_count,
            normal_mismatch_count,
            max_position_error_m,
            max_normal_error,
            passed,
        })
    }
}

/// Tolerances for recorded hand validation mesh comparisons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandValidationMeshTolerance {
    /// Maximum accepted vertex-position error in meters.
    pub max_position_error_m: f32,
    /// Maximum accepted vector difference between normals.
    pub max_normal_error: f32,
}

impl Default for HandValidationMeshTolerance {
    fn default() -> Self {
        Self {
            max_position_error_m: 1.0e-4,
            max_normal_error: 1.0e-3,
        }
    }
}

impl HandValidationMeshTolerance {
    fn validate(self) -> Result<(), MatterMeshError> {
        if !self.max_position_error_m.is_finite()
            || self.max_position_error_m < 0.0
            || !self.max_normal_error.is_finite()
            || self.max_normal_error < 0.0
        {
            return Err(MatterMeshError::InvalidHandPayload(
                "hand validation tolerances must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// Result of comparing a skinned hand frame with a validation mesh frame.
#[derive(Clone, Debug, PartialEq)]
pub struct HandValidationMeshComparison {
    /// Vertex count in the expected validation frame.
    pub expected_vertex_count: usize,
    /// Vertex count in the actual validation frame.
    pub actual_vertex_count: usize,
    /// Normal count in the expected validation frame.
    pub expected_normal_count: usize,
    /// Normal count in the actual validation frame.
    pub actual_normal_count: usize,
    /// Whether topology keys matched exactly.
    pub topology_matched: bool,
    /// Number of compared vertices outside the position tolerance.
    pub position_mismatch_count: usize,
    /// Number of compared normals outside the normal tolerance.
    pub normal_mismatch_count: usize,
    /// Largest vertex-position error in meters.
    pub max_position_error_m: f32,
    /// Largest normal vector difference.
    pub max_normal_error: f32,
    /// Whether topology, counts, and tolerances all passed.
    pub passed: bool,
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

fn validate_joint_metadata(
    parents: &[i16],
    radii: &[f32],
    bind_poses: &[HandJointPose],
) -> Result<(), MatterMeshError> {
    let joint_count = parents.len().max(radii.len()).max(bind_poses.len());
    if !parents.is_empty() && parents.len() != joint_count {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint parent count must match joint count",
        ));
    }
    if !radii.is_empty() && radii.len() != joint_count {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint radii must match joint count",
        ));
    }
    if !bind_poses.is_empty()
        && (bind_poses.len() != joint_count || !bind_poses.iter().all(HandJointPose::is_valid))
    {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint bind poses must match joint count and be finite",
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
        if parent >= 0
            && usize::try_from(parent)
                .map_or(true, |parent| parent >= joint_count || parent == index)
        {
            return Err(MatterMeshError::InvalidHandPayload(
                "joint parent indices must refer to another in-range joint",
            ));
        }
    }
    Ok(())
}

fn validate_skinning_metadata(
    joint_indices: &[[u16; 4]],
    joint_weights: &[[f32; 4]],
    vertex_count: usize,
    joint_count: usize,
) -> Result<(), MatterMeshError> {
    if joint_indices.is_empty() && joint_weights.is_empty() {
        return Ok(());
    }
    if joint_indices.len() != vertex_count || joint_weights.len() != vertex_count {
        return Err(MatterMeshError::InvalidHandPayload(
            "skinning metadata must match vertex count",
        ));
    }
    for (indices, weights) in joint_indices.iter().zip(joint_weights.iter()) {
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
        for slot in 0..4 {
            if weights[slot] > 0.0 && usize::from(indices[slot]) >= joint_count {
                return Err(MatterMeshError::InvalidHandPayload(
                    "skinning joint indices must reference existing joints",
                ));
            }
        }
    }
    Ok(())
}

fn validate_rig_frame_match(
    rig: &HandRigCapture,
    frame: &HandJointFrame,
) -> Result<(), MatterMeshError> {
    if rig.handedness != frame.handedness
        || rig.reference_space != frame.reference_space
        || rig.source != frame.source
    {
        return Err(MatterMeshError::InvalidHandPayload(
            "rig and joint frame metadata must match",
        ));
    }
    Ok(())
}

fn validate_full_bind_joint_frame(
    rig: &HandRigCapture,
    frame: &HandJointFrame,
) -> Result<(), MatterMeshError> {
    if rig.joint_bind_poses.is_empty() {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint bind poses are required for skinning",
        ));
    }
    if frame.poses.len() != rig.joint_bind_poses.len() {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint frame must contain one pose per bind joint",
        ));
    }
    Ok(())
}

fn selected_vertex_index(vertex_count: usize, sample_count: usize, sample_index: usize) -> usize {
    if sample_count <= 1 {
        0
    } else {
        sample_index * (vertex_count - 1) / (sample_count - 1)
    }
}

fn joint_skinning_matrix(
    bind_pose: &HandJointPose,
    joint_pose: &HandJointPose,
) -> Result<[[f32; 4]; 4], MatterMeshError> {
    let origin = joint_skinning_transform_point(bind_pose, joint_pose, Vec3::ZERO)?;
    let x_axis =
        joint_skinning_transform_point(bind_pose, joint_pose, Vec3::new(1.0, 0.0, 0.0))? - origin;
    let y_axis =
        joint_skinning_transform_point(bind_pose, joint_pose, Vec3::new(0.0, 1.0, 0.0))? - origin;
    let z_axis =
        joint_skinning_transform_point(bind_pose, joint_pose, Vec3::new(0.0, 0.0, 1.0))? - origin;

    Ok([
        [x_axis.x, y_axis.x, z_axis.x, origin.x],
        [x_axis.y, y_axis.y, z_axis.y, origin.y],
        [x_axis.z, y_axis.z, z_axis.z, origin.z],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

fn joint_skinning_transform_point(
    bind_pose: &HandJointPose,
    joint_pose: &HandJointPose,
    point: Vec3,
) -> Result<Vec3, MatterMeshError> {
    transform_point(joint_pose, inverse_transform_point(bind_pose, point)?)
}

fn identity_matrix4() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn transform_point(pose: &HandJointPose, point: Vec3) -> Result<Vec3, MatterMeshError> {
    Ok(rotate_by_quat(pose.orientation_xyzw, point)? + pose.position)
}

fn inverse_transform_point(pose: &HandJointPose, point: Vec3) -> Result<Vec3, MatterMeshError> {
    rotate_by_quat(quat_inverse(pose.orientation_xyzw)?, point - pose.position)
}

fn rotate_vec3(pose: &HandJointPose, vector: Vec3) -> Result<Vec3, MatterMeshError> {
    rotate_by_quat(pose.orientation_xyzw, vector)
}

fn inverse_rotate_vec3(pose: &HandJointPose, vector: Vec3) -> Result<Vec3, MatterMeshError> {
    rotate_by_quat(quat_inverse(pose.orientation_xyzw)?, vector)
}

fn rotate_by_quat(quat: [f32; 4], vector: Vec3) -> Result<Vec3, MatterMeshError> {
    let [x, y, z, w] = normalize_quat(quat)?;
    let q_vec = Vec3::new(x, y, z);
    let uv = q_vec.cross(vector);
    let uuv = q_vec.cross(uv);
    Ok(vector + uv * (2.0 * w) + uuv * 2.0)
}

fn quat_inverse(quat: [f32; 4]) -> Result<[f32; 4], MatterMeshError> {
    let [x, y, z, w] = normalize_quat(quat)?;
    Ok([-x, -y, -z, w])
}

fn normalize_quat(quat: [f32; 4]) -> Result<[f32; 4], MatterMeshError> {
    let length_squared: f32 = quat.iter().map(|value| value * value).sum();
    if !length_squared.is_finite() || length_squared <= 1.0e-12 {
        return Err(MatterMeshError::InvalidHandPayload(
            "joint orientation must be finite and non-zero",
        ));
    }
    let scale = 1.0 / length_squared.sqrt();
    Ok([
        quat[0] * scale,
        quat[1] * scale,
        quat[2] * scale,
        quat[3] * scale,
    ])
}

fn normalize_vec3(vector: Vec3) -> Result<Vec3, MatterMeshError> {
    if !vector.is_finite() {
        return Err(MatterMeshError::InvalidHandPayload(
            "skinned normal must be finite",
        ));
    }
    let length = vector.length();
    if length <= 1.0e-12 {
        return Err(MatterMeshError::InvalidHandPayload(
            "skinned normal must be non-zero",
        ));
    }
    Ok(vector / length)
}
