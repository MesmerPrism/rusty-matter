use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;

use crate::{
    MatterFieldError, SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect,
    SurfaceFieldRuntime, SurfaceFieldRuntimeConfig, SurfaceFieldState, SurfaceFieldSubstrate,
    SurfaceScalarField, SurfaceScalarFieldKind, SurfaceVectorField, SurfaceVectorFieldKind,
    SURFACE_FIELD_SUBSTRATE_SCHEMA_ID,
};

#[test]
fn substrate_from_mesh_sample_set_preserves_neighbor_tiers() {
    let substrate = test_substrate();

    assert_eq!(substrate.schema_id, SURFACE_FIELD_SUBSTRATE_SCHEMA_ID);
    assert_eq!(substrate.node_count(), 10);
    assert!(substrate.first_tier_edge_count() > 0);
    assert!(substrate.second_tier_edge_count() > 0);
    substrate.validate().expect("substrate validates");
}

#[test]
fn field_state_validates_surface_field_buffers() {
    let substrate = test_substrate();
    let state = test_state(&substrate);

    assert_eq!(state.scalar_fields.len(), 3);
    assert_eq!(state.vector_fields.len(), 1);
    assert!(state.scalar_field("field.vmem_like").is_some());
    assert!(state.vector_field("field.polarity").is_some());
    state.validate().expect("state validates");
}

#[test]
fn runtime_summary_validates_f1_contracts_without_dynamics() {
    let substrate = test_substrate();
    let state = test_state(&substrate);
    let config = SurfaceFieldRuntimeConfig::default();
    let runtime = SurfaceFieldRuntime::new(config).expect("config validates");
    let perturbations = vec![
        SurfaceFieldPerturbation::new(
            "perturbation.wound.center",
            Some("field.wound_signal".to_owned()),
            vec![0, 1, 2],
            SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
        ),
        SurfaceFieldPerturbation::new(
            "perturbation.polarity.invert",
            Some("field.polarity".to_owned()),
            vec![3, 4],
            SurfaceFieldPerturbationEffect::PolarityInversion,
        ),
    ];

    let summary = runtime
        .validate_contracts(
            "summary.surface_field.contracts",
            &substrate,
            &state,
            &perturbations,
        )
        .expect("summary validates");

    assert_eq!(summary.step_count, 0);
    assert_eq!(summary.node_count, substrate.node_count());
    assert_eq!(summary.perturbation_count, 2);
    assert_eq!(summary.scalar_min, Some(0.0));
    assert_eq!(summary.scalar_max, Some(0.5));
    assert_eq!(summary.max_vector_length, Some(1.0));
}

#[test]
fn damaged_scalar_buffer_length_is_rejected() {
    let substrate = test_substrate();
    let mut scalar =
        SurfaceScalarField::constant("field.vmem_like", SurfaceScalarFieldKind::VmemLike, 3, 0.0);
    scalar.values.push(0.1);
    let error = SurfaceFieldState::new("state.invalid", &substrate, vec![scalar], Vec::new())
        .expect_err("bad scalar length rejects");

    assert!(matches!(error, MatterFieldError::NodeCountMismatch { .. }));
}

#[test]
fn damaged_non_finite_vector_is_rejected() {
    let substrate = test_substrate();
    let vector = SurfaceVectorField::constant(
        "field.polarity",
        SurfaceVectorFieldKind::Polarity,
        substrate.node_count(),
        Vec3::new(f32::INFINITY, 0.0, 0.0),
    );
    let error = SurfaceFieldState::new("state.invalid", &substrate, Vec::new(), vec![vector])
        .expect_err("non-finite vector rejects");

    assert!(matches!(error, MatterFieldError::NonFiniteVector { .. }));
}

#[test]
fn damaged_neighbor_target_is_rejected() {
    let mut substrate = test_substrate();
    let node_count = substrate.node_count();
    substrate.nodes[0].first_tier_neighbors.push(node_count);
    let error = substrate.validate().expect_err("bad neighbor rejects");

    assert!(matches!(
        error,
        MatterFieldError::InvalidNeighbor {
            node_index: 0,
            neighbor_index
        } if neighbor_index == node_count
    ));
}

#[test]
fn damaged_perturbation_target_is_rejected() {
    let substrate = test_substrate();
    let perturbation = SurfaceFieldPerturbation::new(
        "perturbation.invalid",
        Some("field.wound_signal".to_owned()),
        vec![substrate.node_count()],
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
    );
    let error = perturbation
        .validate(substrate.node_count())
        .expect_err("bad perturbation target rejects");

    assert!(matches!(
        error,
        MatterFieldError::InvalidPerturbationNode { .. }
    ));
}

#[test]
fn damaged_runtime_config_is_rejected() {
    let config = SurfaceFieldRuntimeConfig {
        fixed_step_seconds: 0.0,
        ..SurfaceFieldRuntimeConfig::default()
    };
    let error = SurfaceFieldRuntime::new(config).expect_err("bad config rejects");

    assert!(matches!(error, MatterFieldError::InvalidRuntimeConfig(_)));
}

fn test_substrate() -> SurfaceFieldSubstrate {
    let surface = unit_square_surface();
    let config = MeshSurfaceSampleConfig {
        sample_config_id: "mesh.surface_sample.field_tests".to_owned(),
        sample_set_id: "mesh.surface_samples.field_tests".to_owned(),
        point_count: 10,
        first_tier_neighbor_count: 3,
        second_tier_neighbor_count: 3,
        pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
        ..MeshSurfaceSampleConfig::default()
    };
    let samples = surface
        .sample_points(&config)
        .expect("unit square samples validate");
    SurfaceFieldSubstrate::from_sample_set("fields.substrate.unit_square", &samples)
        .expect("field substrate validates")
}

fn test_state(substrate: &SurfaceFieldSubstrate) -> SurfaceFieldState {
    let node_count = substrate.node_count();
    let scalars = vec![
        SurfaceScalarField::constant(
            "field.vmem_like",
            SurfaceScalarFieldKind::VmemLike,
            node_count,
            0.5,
        ),
        SurfaceScalarField::constant(
            "field.wound_signal",
            SurfaceScalarFieldKind::WoundSignal,
            node_count,
            0.0,
        ),
        SurfaceScalarField::constant(
            "field.morphogen",
            SurfaceScalarFieldKind::Morphogen,
            node_count,
            0.25,
        ),
    ];
    let vectors = vec![SurfaceVectorField::constant(
        "field.polarity",
        SurfaceVectorFieldKind::Polarity,
        node_count,
        Vec3::new(1.0, 0.0, 0.0),
    )];
    SurfaceFieldState::new(
        "state.surface_field.unit_square",
        substrate,
        scalars,
        vectors,
    )
    .expect("state validates")
}

fn unit_square_surface() -> TriangleMeshSurface {
    TriangleMeshSurface::new(
        "mesh.unit_square_surface",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    )
}
