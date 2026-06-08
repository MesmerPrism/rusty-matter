use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;

use crate::{
    BioelectricCircuitConfig, BioelectricCircuitRuntime, BioelectricCircuitState,
    BioelectricConductanceEdge, BioelectricCurrentKind, BioelectricCurrentTerm, BioelectricGate,
    BioelectricGateSource, BioelectricMemoryState, BioelectricReadoutLayer,
    BioelectricVoltageField, BioelectricVoltageUnit, MatterFieldError, SurfaceFieldDebugFrame,
    SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect, SurfaceFieldRuntime,
    SurfaceFieldRuntimeConfig, SurfaceFieldState, SurfaceFieldSubstrate, SurfaceScalarField,
    SurfaceScalarFieldKind, SurfaceVectorField, SurfaceVectorFieldKind,
    BIOELECTRIC_CIRCUIT_STATE_SCHEMA_ID, SURFACE_FIELD_SUBSTRATE_SCHEMA_ID,
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
fn debug_frame_exposes_nodes_edges_fields_and_perturbations() {
    let substrate = test_substrate();
    let state = test_state(&substrate);
    let perturbations = vec![SurfaceFieldPerturbation::new(
        "perturbation.wound.center",
        Some("field.wound_signal".to_owned()),
        vec![0, 1, 2],
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
    )];

    let frame = SurfaceFieldDebugFrame::from_contracts(
        "debug.fields.unit_square",
        &substrate,
        &state,
        &perturbations,
    )
    .expect("debug frame validates");

    assert_eq!(frame.nodes.len(), substrate.node_count());
    assert_eq!(
        frame.edges.len(),
        substrate.first_tier_edge_count() + substrate.second_tier_edge_count()
    );
    assert_eq!(frame.scalar_layers.len(), 3);
    assert_eq!(frame.vector_layers.len(), 1);
    assert_eq!(frame.perturbation_regions.len(), 1);
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

#[test]
fn dynamics_plan_uses_sparse_neighbor_links() {
    let substrate = test_substrate();
    let runtime =
        SurfaceFieldRuntime::new(SurfaceFieldRuntimeConfig::default()).expect("runtime config");

    let plan = runtime
        .dynamics_plan(&substrate)
        .expect("sparse plan validates");

    assert_eq!(plan.node_count, substrate.node_count());
    assert_eq!(
        plan.directed_link_count,
        substrate.first_tier_edge_count() + substrate.second_tier_edge_count()
    );
    assert!(plan.links.iter().all(|links| links.len() <= 6));
}

#[test]
fn runtime_debug_sequence_diffuses_scalars_and_bounds_vectors() {
    let substrate = test_substrate();
    let state = test_state(&substrate);
    let runtime =
        SurfaceFieldRuntime::new(SurfaceFieldRuntimeConfig::default()).expect("runtime config");
    let mut wound = SurfaceFieldPerturbation::new(
        "perturbation.wound.dynamic_test",
        Some("field.wound_signal".to_owned()),
        vec![0, 1, 2],
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
    );
    wound.duration_steps = 6;

    let sequence = runtime
        .run_debug_sequence(
            "sequence.surface_field.dynamic_test",
            &substrate,
            &state,
            &[wound],
            18,
            3,
        )
        .expect("dynamic sequence validates");

    assert_eq!(sequence.step_count, 18);
    assert_eq!(sequence.frames.len(), 7);
    assert_eq!(sequence.diagnostics.len(), 18);
    assert!(sequence
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.neighbor_links_visited > 0));

    let initial_wound_count = wound_signal_count(&sequence.frames[0], 0.01);
    let final_wound_count = wound_signal_count(sequence.frames.last().expect("final frame"), 0.01);
    assert_eq!(initial_wound_count, 0);
    assert!(final_wound_count > 3);
    assert!(sequence
        .summary
        .max_vector_length
        .is_some_and(|length| length <= 1.0));
}

#[test]
fn damaged_dynamic_step_count_is_rejected() {
    let substrate = test_substrate();
    let state = test_state(&substrate);
    let config = SurfaceFieldRuntimeConfig {
        max_steps_per_run: 4,
        ..SurfaceFieldRuntimeConfig::default()
    };
    let runtime = SurfaceFieldRuntime::new(config).expect("runtime config");
    let error = runtime
        .run_debug_sequence("sequence.invalid", &substrate, &state, &[], 5, 1)
        .expect_err("too many steps reject");

    assert!(matches!(error, MatterFieldError::InvalidRuntimeConfig(_)));
}

#[test]
fn damaged_dynamic_perturbation_target_is_rejected() {
    let substrate = test_substrate();
    let state = test_state(&substrate);
    let runtime =
        SurfaceFieldRuntime::new(SurfaceFieldRuntimeConfig::default()).expect("runtime config");
    let perturbation = SurfaceFieldPerturbation::new(
        "perturbation.missing_target",
        Some("field.missing".to_owned()),
        vec![0],
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
    );
    let error = runtime
        .run_debug_sequence(
            "sequence.invalid",
            &substrate,
            &state,
            &[perturbation],
            2,
            1,
        )
        .expect_err("missing target rejects");

    assert!(matches!(error, MatterFieldError::InvalidPerturbation(_)));
}

#[test]
fn bioelectric_circuit_contracts_validate_over_surface_nodes() {
    let substrate = test_substrate();
    let circuit = test_circuit_state(&substrate);
    let runtime =
        BioelectricCircuitRuntime::new(BioelectricCircuitConfig::default()).expect("config");

    let diagnostics = runtime
        .validate_contracts(&substrate, &circuit)
        .expect("bioelectric contracts validate");

    assert_eq!(circuit.schema_id, BIOELECTRIC_CIRCUIT_STATE_SCHEMA_ID);
    assert_eq!(circuit.node_count, substrate.node_count());
    assert_eq!(circuit.voltage.values.len(), substrate.node_count());
    assert_eq!(diagnostics.updated_nodes, substrate.node_count());
    assert_eq!(diagnostics.visited_edges, circuit.conductance_edges.len());
}

#[test]
fn bioelectric_circuit_step_updates_voltage_gates_memory_and_readout() {
    let substrate = test_substrate();
    let mut circuit = test_circuit_state(&substrate);
    circuit.voltage.values[0] = 0.45;
    circuit.voltage.values[1] = -0.1;
    circuit.memory.as_mut().expect("memory").values[0] = 0.48;
    let initial_voltage = circuit.voltage.values.clone();
    let initial_conductance = circuit.conductance_edges[0].conductance;
    let initial_readout = circuit.readout_layers[0].values[0];
    let runtime =
        BioelectricCircuitRuntime::new(BioelectricCircuitConfig::default()).expect("config");

    let diagnostics = runtime
        .step_fixed(&substrate, &mut circuit, 0)
        .expect("bioelectric step validates");

    assert!(diagnostics.visited_edges > 0);
    assert!(diagnostics.active_current_terms > 0);
    assert!(diagnostics.active_gates > 0);
    assert!(diagnostics.max_voltage_delta > 0.0);
    assert_ne!(circuit.voltage.values, initial_voltage);
    assert_ne!(
        circuit.conductance_edges[0].conductance,
        initial_conductance
    );
    assert!(circuit.memory.expect("memory").values[0] > 0.48);
    assert_ne!(circuit.readout_layers[0].values[0], initial_readout);
}

#[test]
fn bioelectric_memory_hysteresis_persists_between_thresholds() {
    let substrate = test_substrate();
    let mut circuit = test_circuit_state(&substrate);
    circuit.voltage.values[0] = 0.42;
    let runtime =
        BioelectricCircuitRuntime::new(BioelectricCircuitConfig::default()).expect("config");

    runtime
        .step_fixed(&substrate, &mut circuit, 0)
        .expect("activation step validates");
    let activated = circuit.memory.as_ref().expect("memory").values[0];
    circuit.voltage.values[0] = 0.0;
    runtime
        .step_fixed(&substrate, &mut circuit, 1)
        .expect("hold step validates");
    let held = circuit.memory.as_ref().expect("memory").values[0];

    assert!(activated > 0.0);
    assert_eq!(held, activated);
}

#[test]
fn damaged_bioelectric_voltage_count_is_rejected() {
    let substrate = test_substrate();
    let node_count = substrate.node_count();
    let voltage = BioelectricVoltageField::constant(
        "field.bioelectric_voltage",
        BioelectricVoltageUnit::Normalized,
        node_count.saturating_sub(1),
        0.0,
        0.0,
    );
    let edges = BioelectricConductanceEdge::from_substrate_neighbors(&substrate, 0.2, 0.05, None)
        .expect("edges validate");
    let error = BioelectricCircuitState::new(
        "circuit.invalid_voltage",
        &substrate,
        voltage,
        edges,
        Vec::new(),
        None,
        Vec::new(),
    )
    .expect_err("bad voltage length rejects");

    assert!(matches!(error, MatterFieldError::NodeCountMismatch { .. }));
}

#[test]
fn damaged_bioelectric_conductance_target_is_rejected() {
    let substrate = test_substrate();
    let node_count = substrate.node_count();
    let voltage = BioelectricVoltageField::constant(
        "field.bioelectric_voltage",
        BioelectricVoltageUnit::Normalized,
        node_count,
        0.0,
        0.0,
    );
    let bad_edge =
        BioelectricConductanceEdge::new("conductance.invalid", 0, node_count, 1, 0.2, None);
    let error = BioelectricCircuitState::new(
        "circuit.invalid_edge",
        &substrate,
        voltage,
        vec![bad_edge],
        Vec::new(),
        None,
        Vec::new(),
    )
    .expect_err("bad conductance target rejects");

    assert!(matches!(error, MatterFieldError::InvalidNeighbor { .. }));
}

#[test]
fn damaged_bioelectric_current_target_is_rejected() {
    let substrate = test_substrate();
    let mut circuit = test_circuit_state(&substrate);
    circuit.current_terms.push(BioelectricCurrentTerm::new(
        "current.invalid_target",
        vec![substrate.node_count()],
        BioelectricCurrentKind::Constant { current: 0.1 },
    ));
    let error = circuit.validate().expect_err("bad current target rejects");

    assert!(matches!(
        error,
        MatterFieldError::InvalidPerturbationNode { .. }
    ));
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

fn test_circuit_state(substrate: &SurfaceFieldSubstrate) -> BioelectricCircuitState {
    let node_count = substrate.node_count();
    let voltage_values = substrate
        .nodes
        .iter()
        .map(|node| (node.position.x - 0.5) * 0.35)
        .collect::<Vec<_>>();
    let gate = BioelectricGate::new(
        "gate.voltage_difference.opens",
        BioelectricGateSource::VoltageDifference,
        0.08,
        0.02,
        0.35,
        1.5,
    );
    let conductance_edges =
        BioelectricConductanceEdge::from_substrate_neighbors(&substrate, 0.18, 0.04, Some(gate))
            .expect("conductance edges validate");
    let mut transient_source = BioelectricCurrentTerm::new(
        "current.transient_depolarizing_source",
        vec![0],
        BioelectricCurrentKind::Constant { current: 0.75 },
    );
    transient_source.duration_steps = 3;
    let current_terms = vec![
        BioelectricCurrentTerm::new(
            "current.leak_to_rest",
            Vec::new(),
            BioelectricCurrentKind::Leak {
                conductance: 0.18,
                reversal_voltage: 0.0,
            },
        ),
        BioelectricCurrentTerm::new(
            "current.pump_to_rest",
            Vec::new(),
            BioelectricCurrentKind::Pump {
                rate: 0.12,
                target_voltage: 0.0,
            },
        ),
        BioelectricCurrentTerm::new(
            "current.generic_voltage_gate",
            Vec::new(),
            BioelectricCurrentKind::VoltageGated {
                max_conductance: 0.08,
                reversal_voltage: -0.3,
                threshold: 0.18,
                slope: 0.06,
            },
        ),
        transient_source,
    ];
    let memory = BioelectricMemoryState::zeroed(
        "memory.hysteresis.synthetic",
        node_count,
        0.28,
        -0.18,
        2.4,
        0.8,
    );
    let readout = BioelectricReadoutLayer::new(
        "readout.morphogen_like_voltage",
        vec![0.0; node_count],
        0.75,
        0.5,
        0.1,
        1.6,
        -1.0,
        1.0,
    );
    BioelectricCircuitState::new(
        "circuit.bioelectric.unit_square",
        substrate,
        BioelectricVoltageField::new(
            "field.bioelectric_voltage",
            BioelectricVoltageUnit::Normalized,
            0.0,
            voltage_values,
        ),
        conductance_edges,
        current_terms,
        Some(memory),
        vec![readout],
    )
    .expect("bioelectric circuit validates")
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

fn wound_signal_count(frame: &SurfaceFieldDebugFrame, threshold: f32) -> usize {
    frame
        .scalar_layers
        .iter()
        .find(|layer| layer.field_id == "field.wound_signal")
        .expect("wound layer")
        .values
        .iter()
        .filter(|value| **value > threshold)
        .count()
}
