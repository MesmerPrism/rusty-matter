use rusty_matter_fields::{
    MatterFieldError, SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect, SurfaceFieldState,
    SurfaceFieldSubstrate, SurfaceScalarField, SurfaceScalarFieldKind,
};
use rusty_matter_mesh::{
    HandValidationMeshFrame, Handedness, MatterMeshError, MeshCoordinateFrameConfig,
    MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface,
};
use rusty_matter_model::{MatterModelError, TriangleMeshSnapshot, Vec3};
use rusty_matter_particles::{
    ParticleError, ParticleInfluenceMode, ParticleInfluencePoint, ParticleInteractionBody,
};
use rusty_matter_sdf::{
    build_sdf_from_mesh, MeshSdfSignMode, MeshToSdfConfig, PackedSdfGrid, SdfError,
};

use crate::error::CliError;
use crate::mesh::unit_square_surface;
use crate::sdf::unit_triangle_mesh;
use crate::summary::DamagedFixtureReport;

pub(crate) struct DamagedArtifact {
    pub(crate) path: &'static str,
    pub(crate) report: DamagedFixtureReport,
}

pub(crate) fn damaged_fixture_reports() -> Result<Vec<DamagedArtifact>, CliError> {
    let field_substrate = unit_surface_field_substrate()?;
    let field_node_count = field_substrate.node_count();
    Ok(vec![
        damaged_report(
            "fixtures/damaged/invalid-mesh-index.json",
            "fixture.damaged.invalid_mesh_index.v1",
            "damaged.mesh.invalid_index",
            "model.index_out_of_range",
            build_sdf_from_mesh(
                &TriangleMeshSnapshot::new(
                    "mesh.invalid_index",
                    vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)],
                    vec![[0, 1, 2]],
                ),
                MeshToSdfConfig::default(),
            ),
        )?,
        damaged_report(
            "fixtures/damaged/invalid-voxel-size.json",
            "fixture.damaged.invalid_voxel_size.v1",
            "damaged.sdf.invalid_voxel_size",
            "sdf.invalid_voxel_size",
            build_sdf_from_mesh(
                &unit_triangle_mesh(),
                MeshToSdfConfig {
                    voxel_size: 0.0,
                    ..MeshToSdfConfig::default()
                },
            ),
        )?,
        damaged_report(
            "fixtures/damaged/voxel-budget-overflow.json",
            "fixture.damaged.voxel_budget_overflow.v1",
            "damaged.sdf.voxel_budget_overflow",
            "sdf.voxel_budget_exceeded",
            build_sdf_from_mesh(
                &unit_triangle_mesh(),
                MeshToSdfConfig {
                    voxel_size: 0.01,
                    padding_voxels: 1,
                    max_voxels: 10,
                    sign_mode: MeshSdfSignMode::UnsignedOnly,
                },
            ),
        )?,
        damaged_particle_report(
            "fixtures/damaged/invalid-particle-influence.json",
            "fixture.damaged.invalid_particle_influence.v1",
            "damaged.particle.invalid_influence",
            "particle.invalid_influence_config",
            ParticleInfluencePoint::new(
                "influence.invalid_radius",
                Vec3::ZERO,
                -1.0,
                1.0,
                ParticleInfluenceMode::Attract,
            )
            .validate(),
        )?,
        damaged_particle_report(
            "fixtures/damaged/invalid-particle-body.json",
            "fixture.damaged.invalid_particle_body.v1",
            "damaged.particle.invalid_body",
            "particle.invalid_body_config",
            ParticleInteractionBody::axis_aligned_box(
                "body.invalid_bounds",
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::ZERO,
            )
            .validate(),
        )?,
        damaged_mesh_report(
            "fixtures/damaged/invalid-mesh-surface-index.json",
            "fixture.damaged.invalid_mesh_surface_index.v1",
            "damaged.mesh_surface.invalid_index",
            "mesh.index_out_of_range",
            TriangleMeshSurface::new(
                "mesh.invalid_surface_index",
                vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)],
                vec![[0, 1, 2]],
            )
            .validate(),
        )?,
        damaged_mesh_report(
            "fixtures/damaged/invalid-coordinate-frame-config.json",
            "fixture.damaged.invalid_coordinate_frame_config.v1",
            "damaged.mesh.invalid_coordinate_frame_config",
            "mesh.invalid_coordinate_frame_config",
            MeshCoordinateFrameConfig {
                max_displacement: Vec3::new(-0.01, 0.01, 0.01),
                ..MeshCoordinateFrameConfig::default()
            }
            .validate(),
        )?,
        damaged_mesh_report(
            "fixtures/damaged/invalid-hand-validation-mesh-frame.json",
            "fixture.damaged.invalid_hand_validation_mesh_frame.v1",
            "damaged.hand.invalid_validation_mesh_frame",
            "mesh.invalid_hand_payload",
            HandValidationMeshFrame::from_surface(
                "hand.validation_mesh.invalid",
                Handedness::Left,
                "local_floor",
                "meta.hand_tracking_mesh",
                -1.0,
                unit_square_surface(),
            )
            .validate(),
        )?,
        damaged_field_report(
            "fixtures/damaged/invalid-surface-field-state.json",
            "fixture.damaged.invalid_surface_field_state.v1",
            "damaged.fields.invalid_state",
            "field.node_count_mismatch",
            SurfaceFieldState::new(
                "state.invalid_field_count",
                &field_substrate,
                vec![SurfaceScalarField::constant(
                    "field.vmem_like",
                    SurfaceScalarFieldKind::VmemLike,
                    field_node_count.saturating_sub(1),
                    0.0,
                )],
                Vec::new(),
            )
            .map(|_| ()),
        )?,
        damaged_field_report(
            "fixtures/damaged/invalid-surface-field-perturbation.json",
            "fixture.damaged.invalid_surface_field_perturbation.v1",
            "damaged.fields.invalid_perturbation",
            "field.invalid_perturbation_node",
            SurfaceFieldPerturbation::new(
                "perturbation.invalid_target",
                Some("field.wound_signal".to_owned()),
                vec![field_node_count],
                SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
            )
            .validate(field_node_count),
        )?,
    ])
}

fn damaged_report(
    path: &'static str,
    fixture_id: impl Into<String>,
    damaged_input_id: impl Into<String>,
    expected_rejection_code: &'static str,
    result: Result<PackedSdfGrid, SdfError>,
) -> Result<DamagedArtifact, CliError> {
    let fixture_id = fixture_id.into();
    let Err(error) = result else {
        return Err(CliError::ExpectedRejection { fixture_id });
    };
    let actual_rejection_code = sdf_rejection_code(&error);
    if actual_rejection_code != expected_rejection_code {
        return Err(CliError::UnexpectedRejection {
            expected: expected_rejection_code.to_owned(),
            actual: actual_rejection_code,
        });
    }

    Ok(DamagedArtifact {
        path,
        report: DamagedFixtureReport {
            schema_id: "rusty.matter.fixture.damaged_input_report.v1".to_owned(),
            fixture_id,
            damaged_input_id: damaged_input_id.into(),
            expected_rejection_code: expected_rejection_code.to_owned(),
            actual_rejection_code,
            message: error.to_string(),
        },
    })
}

fn damaged_particle_report(
    path: &'static str,
    fixture_id: impl Into<String>,
    damaged_input_id: impl Into<String>,
    expected_rejection_code: &'static str,
    result: Result<(), ParticleError>,
) -> Result<DamagedArtifact, CliError> {
    let fixture_id = fixture_id.into();
    let Err(error) = result else {
        return Err(CliError::ExpectedRejection { fixture_id });
    };
    let actual_rejection_code = particle_rejection_code(&error);
    if actual_rejection_code != expected_rejection_code {
        return Err(CliError::UnexpectedRejection {
            expected: expected_rejection_code.to_owned(),
            actual: actual_rejection_code,
        });
    }

    Ok(DamagedArtifact {
        path,
        report: DamagedFixtureReport {
            schema_id: "rusty.matter.fixture.damaged_input_report.v1".to_owned(),
            fixture_id,
            damaged_input_id: damaged_input_id.into(),
            expected_rejection_code: expected_rejection_code.to_owned(),
            actual_rejection_code,
            message: error.to_string(),
        },
    })
}

fn damaged_mesh_report(
    path: &'static str,
    fixture_id: impl Into<String>,
    damaged_input_id: impl Into<String>,
    expected_rejection_code: &'static str,
    result: Result<(), MatterMeshError>,
) -> Result<DamagedArtifact, CliError> {
    let fixture_id = fixture_id.into();
    let Err(error) = result else {
        return Err(CliError::ExpectedRejection { fixture_id });
    };
    let actual_rejection_code = mesh_rejection_code(&error);
    if actual_rejection_code != expected_rejection_code {
        return Err(CliError::UnexpectedRejection {
            expected: expected_rejection_code.to_owned(),
            actual: actual_rejection_code,
        });
    }

    Ok(DamagedArtifact {
        path,
        report: DamagedFixtureReport {
            schema_id: "rusty.matter.fixture.damaged_input_report.v1".to_owned(),
            fixture_id,
            damaged_input_id: damaged_input_id.into(),
            expected_rejection_code: expected_rejection_code.to_owned(),
            actual_rejection_code,
            message: error.to_string(),
        },
    })
}

fn damaged_field_report(
    path: &'static str,
    fixture_id: impl Into<String>,
    damaged_input_id: impl Into<String>,
    expected_rejection_code: &'static str,
    result: Result<(), MatterFieldError>,
) -> Result<DamagedArtifact, CliError> {
    let fixture_id = fixture_id.into();
    let Err(error) = result else {
        return Err(CliError::ExpectedRejection { fixture_id });
    };
    let actual_rejection_code = field_rejection_code(&error);
    if actual_rejection_code != expected_rejection_code {
        return Err(CliError::UnexpectedRejection {
            expected: expected_rejection_code.to_owned(),
            actual: actual_rejection_code,
        });
    }

    Ok(DamagedArtifact {
        path,
        report: DamagedFixtureReport {
            schema_id: "rusty.matter.fixture.damaged_input_report.v1".to_owned(),
            fixture_id,
            damaged_input_id: damaged_input_id.into(),
            expected_rejection_code: expected_rejection_code.to_owned(),
            actual_rejection_code,
            message: error.to_string(),
        },
    })
}

fn unit_surface_field_substrate() -> Result<SurfaceFieldSubstrate, CliError> {
    let surface = unit_square_surface();
    let config = MeshSurfaceSampleConfig {
        sample_config_id: "mesh.surface_sample.damaged_field_fixture".to_owned(),
        sample_set_id: "mesh.surface_samples.damaged_field_fixture".to_owned(),
        point_count: 8,
        first_tier_neighbor_count: 2,
        second_tier_neighbor_count: 2,
        seed: 14_003,
        pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
        ..MeshSurfaceSampleConfig::default()
    };
    let samples = surface.sample_points(&config).map_err(CliError::Mesh)?;
    SurfaceFieldSubstrate::from_sample_set("fields.substrate.damaged_fixture", &samples)
        .map_err(CliError::Field)
}

fn sdf_rejection_code(error: &SdfError) -> String {
    match error {
        SdfError::Model(MatterModelError::IndexOutOfRange { .. }) => "model.index_out_of_range",
        SdfError::Model(MatterModelError::DegenerateTriangle { .. }) => "model.degenerate_triangle",
        SdfError::Model(MatterModelError::NonFinitePoint { .. }) => "model.non_finite_point",
        SdfError::Model(_) => "model.validation",
        SdfError::InvalidVoxelSize(_) => "sdf.invalid_voxel_size",
        SdfError::InvalidVoxelBudget => "sdf.invalid_voxel_budget",
        SdfError::VoxelBudgetExceeded { .. } => "sdf.voxel_budget_exceeded",
        SdfError::ZeroDimension => "sdf.zero_dimension",
        SdfError::VoxelCountOverflow => "sdf.voxel_count_overflow",
        SdfError::DistanceCountMismatch { .. } => "sdf.distance_count_mismatch",
        SdfError::NonFiniteDistance { .. } => "sdf.non_finite_distance",
        SdfError::UnexpectedSchema { .. } => "sdf.unexpected_schema",
        SdfError::EmptyGridId => "sdf.empty_grid_id",
        SdfError::NonFiniteOrigin => "sdf.non_finite_origin",
        SdfError::DegenerateTriangle => "sdf.degenerate_triangle",
        SdfError::BatchExecution(_) => "sdf.batch_execution",
    }
    .to_owned()
}

fn particle_rejection_code(error: &ParticleError) -> String {
    match error {
        ParticleError::UnexpectedSchema { .. } => "particle.unexpected_schema",
        ParticleError::EmptyParticleId => "particle.empty_particle_id",
        ParticleError::EmptySetId => "particle.empty_set_id",
        ParticleError::EmptyRenderPayloadId => "particle.empty_render_payload_id",
        ParticleError::EmptyInteractionId => "particle.empty_interaction_id",
        ParticleError::EmptyInteractionsId => "particle.empty_interactions_id",
        ParticleError::EmptyInfluenceId => "particle.empty_influence_id",
        ParticleError::EmptyImpulseId => "particle.empty_impulse_id",
        ParticleError::EmptyBodyId => "particle.empty_body_id",
        ParticleError::EmptyStepConfigId => "particle.empty_step_config_id",
        ParticleError::NonFinitePosition { .. } => "particle.non_finite_position",
        ParticleError::NonFiniteVelocity { .. } => "particle.non_finite_velocity",
        ParticleError::InvalidRadius { .. } => "particle.invalid_radius",
        ParticleError::InvalidInverseMass { .. } => "particle.invalid_inverse_mass",
        ParticleError::InvalidAge { .. } => "particle.invalid_age",
        ParticleError::InvalidSetTime => "particle.invalid_set_time",
        ParticleError::InvalidInteractionConfig(_) => "particle.invalid_sdf_interaction_config",
        ParticleError::InvalidNeighborConfig(_) => "particle.invalid_neighbor_config",
        ParticleError::InvalidInfluenceConfig(_) => "particle.invalid_influence_config",
        ParticleError::InvalidImpulseConfig(_) => "particle.invalid_impulse_config",
        ParticleError::InvalidBodyConfig(_) => "particle.invalid_body_config",
        ParticleError::InvalidExecutionConfig(_) => "particle.invalid_execution_config",
        ParticleError::BatchExecution(_) => "particle.batch_execution",
        ParticleError::InvalidRenderPayload(_) => "particle.invalid_render_payload",
        ParticleError::InvalidSpatialHashCellSize => "particle.invalid_spatial_hash_cell_size",
        ParticleError::InvalidFixedStep => "particle.invalid_fixed_step",
        ParticleError::InvalidMaxSteps => "particle.invalid_max_steps",
    }
    .to_owned()
}

fn field_rejection_code(error: &MatterFieldError) -> String {
    match error {
        MatterFieldError::UnexpectedSchema { .. } => "field.unexpected_schema",
        MatterFieldError::EmptySubstrateId => "field.empty_substrate_id",
        MatterFieldError::EmptyNodeId => "field.empty_node_id",
        MatterFieldError::EmptyFieldId => "field.empty_field_id",
        MatterFieldError::EmptyStateId => "field.empty_state_id",
        MatterFieldError::EmptyPerturbationId => "field.empty_perturbation_id",
        MatterFieldError::EmptyRuntimeConfigId => "field.empty_runtime_config_id",
        MatterFieldError::EmptyRunSummaryId => "field.empty_run_summary_id",
        MatterFieldError::InvalidSubstrate(_) => "field.invalid_substrate",
        MatterFieldError::InvalidField(_) => "field.invalid_field",
        MatterFieldError::InvalidPerturbation(_) => "field.invalid_perturbation",
        MatterFieldError::InvalidRuntimeConfig(_) => "field.invalid_runtime_config",
        MatterFieldError::InvalidRunSummary(_) => "field.invalid_run_summary",
        MatterFieldError::NodeCountMismatch { .. } => "field.node_count_mismatch",
        MatterFieldError::NonFiniteScalar { .. } => "field.non_finite_scalar",
        MatterFieldError::NonFiniteVector { .. } => "field.non_finite_vector",
        MatterFieldError::InvalidNeighbor { .. } => "field.invalid_neighbor",
        MatterFieldError::SelfNeighbor { .. } => "field.self_neighbor",
        MatterFieldError::DuplicateNeighbor { .. } => "field.duplicate_neighbor",
        MatterFieldError::DuplicateFieldId { .. } => "field.duplicate_field_id",
        MatterFieldError::DuplicatePerturbationNode { .. } => "field.duplicate_perturbation_node",
        MatterFieldError::InvalidPerturbationNode { .. } => "field.invalid_perturbation_node",
    }
    .to_owned()
}

fn mesh_rejection_code(error: &MatterMeshError) -> String {
    match error {
        MatterMeshError::UnexpectedSchema { .. } => "mesh.unexpected_schema",
        MatterMeshError::EmptySurfaceId => "mesh.empty_surface_id",
        MatterMeshError::EmptySampleConfigId => "mesh.empty_sample_config_id",
        MatterMeshError::EmptySampleSetId => "mesh.empty_sample_set_id",
        MatterMeshError::EmptyCoordinateMapId => "mesh.empty_coordinate_map_id",
        MatterMeshError::EmptyCoordinateFrameConfigId => "mesh.empty_coordinate_frame_config_id",
        MatterMeshError::EmptyCoordinateFrameSetId => "mesh.empty_coordinate_frame_set_id",
        MatterMeshError::EmptyColliderConfigId => "mesh.empty_collider_config_id",
        MatterMeshError::EmptyHandRigCaptureId => "mesh.empty_hand_rig_capture_id",
        MatterMeshError::EmptyHandJointFrameId => "mesh.empty_hand_joint_frame_id",
        MatterMeshError::EmptyHandFrameId => "mesh.empty_hand_frame_id",
        MatterMeshError::NonFinitePosition { .. } => "mesh.non_finite_position",
        MatterMeshError::DegenerateTriangle { .. } => "mesh.degenerate_triangle",
        MatterMeshError::IndexOutOfRange { .. } => "mesh.index_out_of_range",
        MatterMeshError::InvalidSurface(_) => "mesh.invalid_surface",
        MatterMeshError::InvalidSampleConfig(_) => "mesh.invalid_sample_config",
        MatterMeshError::InvalidColliderConfig(_) => "mesh.invalid_collider_config",
        MatterMeshError::InvalidCoordinateFrameConfig(_) => "mesh.invalid_coordinate_frame_config",
        MatterMeshError::InvalidCoordinateMap(_) => "mesh.invalid_coordinate_map",
        MatterMeshError::InvalidHandPayload(_) => "mesh.invalid_hand_payload",
        MatterMeshError::ChangedTopology => "mesh.changed_topology",
    }
    .to_owned()
}
