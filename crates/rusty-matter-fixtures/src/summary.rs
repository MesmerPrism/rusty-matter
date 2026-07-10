use rusty_matter_model::Vec3;
use rusty_matter_particles::{
    ParticleFixedStepConfig, ParticleRenderPayload, ParticleSet, ParticleSimulationDiagnostics,
};
use rusty_matter_surface_runtime::MatterSurfaceParticleSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct SdfFixtureSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) mesh_id: String,
    pub(crate) grid_id: String,
    pub(crate) dimensions: [u32; 3],
    pub(crate) voxel_size: f32,
    pub(crate) sample_count: usize,
    pub(crate) min_distance: f32,
    pub(crate) max_distance: f32,
    pub(crate) origin: Vec3,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct AdfFixtureSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) field_id: String,
    pub(crate) source_grid_id: String,
    pub(crate) root_origin: Vec3,
    pub(crate) root_extent: f32,
    pub(crate) max_depth: u32,
    pub(crate) source_sample_count: usize,
    pub(crate) cell_count: usize,
    pub(crate) split_count: usize,
    pub(crate) max_level: u32,
    pub(crate) min_cell_distance: f32,
    pub(crate) max_cell_distance: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct DamagedFixtureReport {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) damaged_input_id: String,
    pub(crate) expected_rejection_code: String,
    pub(crate) actual_rejection_code: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct ParticleStepSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) set_id: String,
    pub(crate) particle_count: usize,
    pub(crate) fixed_steps: u32,
    pub(crate) sampled_particles: usize,
    pub(crate) affected_particles: usize,
    pub(crate) rejected_particles: usize,
    pub(crate) clamped_particles: usize,
    pub(crate) neighbor_checks: usize,
    pub(crate) influence_samples: usize,
    pub(crate) impulses_applied: usize,
    pub(crate) body_collisions: usize,
    pub(crate) max_speed: f32,
    pub(crate) first_position: Vec3,
    pub(crate) first_velocity: Vec3,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct ParticleRenderPayloadSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) payload_id: String,
    pub(crate) source_set_id: String,
    pub(crate) sample_count: usize,
    pub(crate) first_particle_id: String,
    pub(crate) first_position: Vec3,
    pub(crate) first_radius: f32,
    pub(crate) first_speed: f32,
    pub(crate) bounds_min: Option<Vec3>,
    pub(crate) bounds_max: Option<Vec3>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParticleContractConformance {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) particle_set: ParticleSet,
    pub(crate) fixed_step: ParticleFixedStepConfig,
    pub(crate) diagnostics: ParticleSimulationDiagnostics,
    pub(crate) render_payload: ParticleRenderPayload,
    pub(crate) surface_snapshot: MatterSurfaceParticleSnapshot,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct MeshSurfaceSampleSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) surface_id: String,
    pub(crate) topology_index_hash: u64,
    pub(crate) vertex_count: usize,
    pub(crate) triangle_count: usize,
    pub(crate) sample_count: usize,
    pub(crate) pattern: String,
    pub(crate) first_position: Vec3,
    pub(crate) first_normal: Vec3,
    pub(crate) first_tier_min: usize,
    pub(crate) first_tier_max: usize,
    pub(crate) second_tier_min: usize,
    pub(crate) second_tier_max: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct MeshCoordinateMapSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) coordinate_map_id: String,
    pub(crate) surface_id: String,
    pub(crate) topology_index_hash: u64,
    pub(crate) sample_count: usize,
    pub(crate) frame_count: usize,
    pub(crate) clamp_mode: String,
    pub(crate) first_anchor: Vec3,
    pub(crate) first_axis_z: Vec3,
    pub(crate) first_displaced_point: Vec3,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct MeshCoordinateMapPackageSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) package_id: String,
    pub(crate) source_id: String,
    pub(crate) source_format: String,
    pub(crate) source_hash: String,
    pub(crate) surface_id: String,
    pub(crate) coordinate_map_id: String,
    pub(crate) topology_index_hash: u64,
    pub(crate) sample_count: usize,
    pub(crate) has_same_surface_neighbors: bool,
    pub(crate) first_anchor: Vec3,
    pub(crate) first_normal: Vec3,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct DynamicColliderSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) surface_id: String,
    pub(crate) status: String,
    pub(crate) vertex_count: usize,
    pub(crate) triangle_count: usize,
    pub(crate) diagnostic_shell_vertex_count: usize,
    pub(crate) diagnostic_shell_triangle_count: usize,
    pub(crate) closest_point: Vec3,
    pub(crate) closest_distance: f32,
    pub(crate) overlaps_probe_sphere: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct HandValidationMeshSummary {
    pub(crate) schema_id: String,
    pub(crate) fixture_id: String,
    pub(crate) frame_id: String,
    pub(crate) handedness: String,
    pub(crate) source: String,
    pub(crate) surface_id: String,
    pub(crate) topology_index_hash: u64,
    pub(crate) vertex_count: usize,
    pub(crate) triangle_count: usize,
}
