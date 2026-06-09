use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;

use crate::{
    bioelectric_node_voltage_neighborhood_targets, BioelectricCircuitConfig,
    BioelectricCircuitEdit, BioelectricCircuitEditOperation, BioelectricCircuitRuntime,
    BioelectricCircuitState, BioelectricConductanceEdge, BioelectricCurrentKind,
    BioelectricCurrentTerm, BioelectricGate, BioelectricGateSource, BioelectricMemoryState,
    BioelectricReadoutLayer, BioelectricVoltageField, BioelectricVoltageUnit, MatterFieldError,
    PlanarianAxisRegion, PlanarianBioelectricPresetConfig, PlanarianBioelectricScenarioKind,
    PlanarianBioelectricScenarioRun, SurfaceFieldDebugFrame, SurfaceFieldPerturbation,
    SurfaceFieldPerturbationEffect, SurfaceFieldRuntime, SurfaceFieldRuntimeConfig,
    SurfaceFieldState, SurfaceFieldSubstrate, SurfaceScalarField, SurfaceScalarFieldKind,
    SurfaceVectorField, SurfaceVectorFieldKind, BIOELECTRIC_CIRCUIT_STATE_SCHEMA_ID,
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
    assert!((circuit.conductance_edges[0].conductance - initial_conductance).abs() > 1.0e-6);
    assert!(circuit.memory.expect("memory").values[0] > 0.48);
    assert!((circuit.readout_layers[0].values[0] - initial_readout).abs() > 1.0e-6);
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
    assert_close(held, activated);
}

#[test]
fn bioelectric_runtime_applies_interactive_edits_with_revisions() {
    let substrate = test_substrate();
    let mut circuit = test_circuit_state(&substrate);
    let runtime =
        BioelectricCircuitRuntime::new(BioelectricCircuitConfig::default()).expect("config");

    let edit = BioelectricCircuitEdit::new(
        "edit.set_voltage",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::SetNodeVoltage {
            node_index: 0,
            voltage: 2.0,
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &edit)
        .expect("voltage edit validates");
    assert!(result.accepted);
    assert_eq!(result.revision_before, 0);
    assert_eq!(result.revision_after, 1);
    assert_eq!(result.clamped_values, 1);
    assert_close(circuit.voltage.values[0], 1.0);

    let edit = BioelectricCircuitEdit::new(
        "edit.set_memory",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::SetNodeMemory {
            node_index: 0,
            memory_value: 0.72,
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &edit)
        .expect("memory edit validates");
    assert!(result.accepted);
    assert_close(circuit.memory.as_ref().expect("memory").values[0], 0.72);

    let edit = BioelectricCircuitEdit::new(
        "edit.gate_threshold",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::SetEdgeGateThreshold {
            edge_index: 0,
            threshold: 0.22,
            slope: Some(0.04),
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &edit)
        .expect("gate edit validates");
    assert!(result.accepted);
    let gate = circuit.conductance_edges[0].gate.as_ref().expect("gate");
    assert_close(gate.threshold, 0.22);
    assert_close(gate.slope, 0.04);

    let edit = BioelectricCircuitEdit::new(
        "edit.scale_conductance",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::ScaleIncidentConductance {
            node_index: 0,
            scale: 0.25,
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &edit)
        .expect("conductance edit validates");
    assert!(result.accepted);
    assert!(!result.affected_edge_indices.is_empty());

    let edit = BioelectricCircuitEdit::new(
        "edit.transient_current",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::AddTransientCurrent {
            term_id: "current.interactive.node0".to_owned(),
            target_node_indices: vec![0],
            current: 0.5,
            start_step: 0,
            duration_steps: 3,
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &edit)
        .expect("current edit validates");
    assert!(result.accepted);
    assert_eq!(
        result.affected_current_term_ids,
        vec!["current.interactive.node0".to_owned()]
    );

    let revision_before_step = circuit.revision;
    runtime
        .step_fixed(&substrate, &mut circuit, 0)
        .expect("step after edits validates");
    assert_eq!(circuit.revision, revision_before_step + 1);
}

#[test]
fn bioelectric_runtime_applies_tiered_neighborhood_voltage_edit() {
    let substrate = test_substrate();
    let mut circuit = test_circuit_state(&substrate);
    let runtime =
        BioelectricCircuitRuntime::new(BioelectricCircuitConfig::default()).expect("config");
    let voltage_before = circuit.voltage.values.clone();
    let targets = bioelectric_node_voltage_neighborhood_targets(&circuit, 0, 1, 0.5)
        .expect("neighborhood targets validate");

    assert!(targets.len() > 1);
    assert!(targets.iter().any(|target| {
        target.node_index == 0 && target.tier == 0 && (target.weight - 1.0).abs() <= 1.0e-6
    }));
    assert!(targets.iter().any(|target| {
        target.node_index != 0 && target.tier == 1 && (target.weight - 0.5).abs() <= 1.0e-6
    }));

    let edit = BioelectricCircuitEdit::new(
        "edit.neighborhood_voltage",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::AddNodeVoltageNeighborhood {
            node_index: 0,
            delta: 0.2,
            max_tier: 1,
            neighbor_falloff: 0.5,
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &edit)
        .expect("neighborhood edit validates");

    assert!(result.accepted);
    assert_eq!(result.affected_node_indices.len(), targets.len());
    for target in targets {
        assert!(result.affected_node_indices.contains(&target.node_index));
        assert_close(
            circuit.voltage.values[target.node_index],
            voltage_before[target.node_index] + 0.2 * target.weight,
        );
    }
}

#[test]
fn damaged_bioelectric_edit_requests_are_rejected() {
    let substrate = test_substrate();
    let mut circuit = test_circuit_state(&substrate);
    let runtime =
        BioelectricCircuitRuntime::new(BioelectricCircuitConfig::default()).expect("config");

    let bad_target = BioelectricCircuitEdit::new(
        "edit.bad_target",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::SetNodeVoltage {
            node_index: substrate.node_count(),
            voltage: 0.1,
        },
    );
    let error = runtime
        .apply_edit(&substrate, &mut circuit, &bad_target)
        .expect_err("bad node target rejects");
    assert!(matches!(
        error,
        MatterFieldError::InvalidPerturbationNode { .. }
    ));

    let non_finite = BioelectricCircuitEdit::new(
        "edit.non_finite",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::SetNodeVoltage {
            node_index: 0,
            voltage: f32::INFINITY,
        },
    );
    let error = runtime
        .apply_edit(&substrate, &mut circuit, &non_finite)
        .expect_err("non-finite voltage rejects");
    assert!(matches!(error, MatterFieldError::InvalidField(_)));

    let stale = BioelectricCircuitEdit::new(
        "edit.stale_revision",
        Some(circuit.revision + 1),
        BioelectricCircuitEditOperation::AddNodeVoltage {
            node_index: 0,
            delta: 0.1,
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &stale)
        .expect("stale edit returns rejected result");
    assert!(!result.accepted);
    assert_eq!(result.revision_before, circuit.revision);
    assert_eq!(result.revision_after, circuit.revision);

    circuit.conductance_edges[0].gate = None;
    let missing_gate = BioelectricCircuitEdit::new(
        "edit.missing_gate",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::SetEdgeGateThreshold {
            edge_index: 0,
            threshold: 0.2,
            slope: None,
        },
    );
    let result = runtime
        .apply_edit(&substrate, &mut circuit, &missing_gate)
        .expect("missing gate returns rejected result");
    assert!(!result.accepted);
    assert_eq!(result.revision_after, circuit.revision);

    let bad_neighborhood_tier = BioelectricCircuitEdit::new(
        "edit.bad_neighborhood_tier",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::AddNodeVoltageNeighborhood {
            node_index: 0,
            delta: 0.1,
            max_tier: 0,
            neighbor_falloff: 0.5,
        },
    );
    let error = runtime
        .apply_edit(&substrate, &mut circuit, &bad_neighborhood_tier)
        .expect_err("zero neighborhood tier rejects");
    assert!(matches!(error, MatterFieldError::InvalidField(_)));

    let bad_neighborhood_falloff = BioelectricCircuitEdit::new(
        "edit.bad_neighborhood_falloff",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::AddNodeVoltageNeighborhood {
            node_index: 0,
            delta: 0.1,
            max_tier: 1,
            neighbor_falloff: 1.25,
        },
    );
    let error = runtime
        .apply_edit(&substrate, &mut circuit, &bad_neighborhood_falloff)
        .expect_err("out-of-range neighborhood falloff rejects");
    assert!(matches!(error, MatterFieldError::InvalidField(_)));
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

#[test]
fn planarian_axis_map_assigns_every_node_to_one_region() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);

    assert_eq!(run.substrate.node_count(), run.axis_map.node_regions.len());
    for region in PlanarianAxisRegion::all() {
        assert!(
            !run.axis_map.nodes_in_region(region).is_empty(),
            "{region:?} should have sampled nodes"
        );
    }
    run.validate().expect("planarian run validates");
}

#[test]
fn planarian_scenario_preserves_source_surface_topology() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);

    assert_eq!(run.source_surface.surface_id, run.substrate.surface_id);
    assert_eq!(
        run.source_surface.topology_key(),
        run.substrate.topology_key
    );
    assert!(run.source_surface.vertex_count() > 0);
    assert!(run.source_surface.triangle_count() > 0);
    for node in &run.substrate.nodes {
        assert!(node.triangle_index < run.source_surface.triangle_count());
        let barycentric_sum: f32 = node.barycentric.iter().sum();
        assert!((barycentric_sum - 1.0).abs() <= 1.0e-4);
    }
    run.validate().expect("planarian run validates");
}

#[test]
fn planarian_default_surface_uses_reviewed_glb_mesh() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);

    assert_eq!(
        run.source_surface.surface_id,
        "mesh.planarian_ap.sketchfab_educational_surface"
    );
    assert_eq!(run.source_surface.vertex_count(), 13_663);
    assert_eq!(run.source_surface.triangle_count(), 23_468);
    assert_eq!(run.surface_provenance.license, "CC-BY-4.0");
    assert_eq!(
        run.surface_provenance.source_sha256,
        "a170a62ba705a81e73dd7fcfb5808431ff1a0b5c0da6322742c1e2c6ce480dda"
    );
}

#[test]
fn planarian_baseline_separates_head_and_tail_readouts() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);
    let final_frame = run.sequence.frames.last().expect("final frame");
    let head_nodes = run.axis_map.nodes_in_region(PlanarianAxisRegion::Head);
    let tail_nodes = run.axis_map.nodes_in_region(PlanarianAxisRegion::Tail);

    let head_identity_at_head = average_readout(
        final_frame,
        "readout.planarian_ap.head_identity",
        &head_nodes,
    );
    let head_identity_at_tail = average_readout(
        final_frame,
        "readout.planarian_ap.head_identity",
        &tail_nodes,
    );
    let tail_identity_at_tail = average_readout(
        final_frame,
        "readout.planarian_ap.tail_identity",
        &tail_nodes,
    );
    let tail_identity_at_head = average_readout(
        final_frame,
        "readout.planarian_ap.tail_identity",
        &head_nodes,
    );

    assert!(head_identity_at_head > head_identity_at_tail + 0.35);
    assert!(tail_identity_at_tail > tail_identity_at_head + 0.35);
}

#[test]
fn planarian_wound_current_is_localized_to_cut_band() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::TransverseCutWound);
    let wound = run
        .initial_circuit
        .current_terms
        .iter()
        .find(|term| term.term_id == "current.planarian_ap.transverse_wound_depolarization")
        .expect("wound current term");
    let target_nodes = &wound.target_node_indices;
    assert!(!target_nodes.is_empty());
    assert!(target_nodes.iter().all(|node_index| {
        let z = run
            .axis_map
            .node_normalized_z(*node_index)
            .expect("node has AP coordinate");
        (z - 0.16).abs() <= 0.11 + 1.0e-5
    }));

    let initial_frame = &run.sequence.frames[0];
    let wound_frame = run
        .sequence
        .frames
        .iter()
        .find(|frame| frame.step_index == 10)
        .expect("wound active frame");
    let wound_delta =
        average_voltage(wound_frame, target_nodes) - average_voltage(initial_frame, target_nodes);
    let outside_nodes = outside_nodes(run.substrate.node_count(), target_nodes);
    let outside_delta = average_voltage(wound_frame, &outside_nodes)
        - average_voltage(initial_frame, &outside_nodes);

    assert!(wound_delta > 0.25);
    assert!(wound_delta > outside_delta + 0.15);
}

#[test]
fn planarian_gap_block_reduces_cross_band_conductance() {
    let baseline = planarian_run(PlanarianBioelectricScenarioKind::Baseline);
    let gap_block = planarian_run(PlanarianBioelectricScenarioKind::GapBlock);

    let baseline_cross = average_cross_cut_conductance(&baseline, 0.16);
    let blocked_cross = average_cross_cut_conductance(&gap_block, 0.16);

    assert!(baseline_cross > 0.0);
    assert!(blocked_cross < baseline_cross * 0.15);
}

#[test]
fn planarian_transient_memory_persists_after_perturbation() {
    let memory = planarian_run(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory);
    let control =
        planarian_run(PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl);
    let posterior_nodes = posterior_nodes(&memory);
    let memory_final = memory.sequence.frames.last().expect("memory final frame");
    let control_final = control.sequence.frames.last().expect("control final frame");

    let memory_head = average_readout(
        memory_final,
        "readout.planarian_ap.head_identity",
        &posterior_nodes,
    );
    let control_head = average_readout(
        control_final,
        "readout.planarian_ap.head_identity",
        &posterior_nodes,
    );
    let memory_average = average_memory(memory_final, &posterior_nodes);

    assert!(memory_average > 0.35);
    assert!(memory_head > control_head + 0.20);
}

#[test]
fn planarian_realtime_edit_changes_selected_node_voltage() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);
    let runtime =
        BioelectricCircuitRuntime::new(run.circuit_config.clone()).expect("planarian runtime");
    let mut circuit = run.initial_circuit.clone();
    let posterior_node = run.axis_map.nodes_in_region(PlanarianAxisRegion::Tail)[0];
    let before = circuit.voltage.values[posterior_node];

    let edit = BioelectricCircuitEdit::new(
        "edit.planarian.posterior_voltage",
        Some(circuit.revision),
        BioelectricCircuitEditOperation::AddNodeVoltage {
            node_index: posterior_node,
            delta: 0.65,
        },
    );
    let result = runtime
        .apply_edit(&run.substrate, &mut circuit, &edit)
        .expect("planarian voltage edit validates");

    assert!(result.accepted);
    assert_eq!(result.affected_node_indices, [posterior_node]);
    assert!(circuit.voltage.values[posterior_node] > before + 0.60);
    runtime
        .step_fixed(&run.substrate, &mut circuit, 0)
        .expect("edited planarian step validates");
    assert_eq!(circuit.revision, result.revision_after + 1);
}

#[test]
fn damaged_planarian_config_and_axis_map_are_rejected() {
    let bad_config = PlanarianBioelectricPresetConfig {
        sample_count: 0,
        ..PlanarianBioelectricPresetConfig::default()
    };
    let error = PlanarianBioelectricScenarioRun::build(
        PlanarianBioelectricScenarioKind::Baseline,
        bad_config,
    )
    .expect_err("bad config rejects");

    assert!(matches!(error, MatterFieldError::InvalidSubstrate(_)));

    let mut run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);
    run.axis_map.node_regions[0].node_index = run.substrate.node_count();
    let error = run.validate().expect_err("bad axis map rejects");

    assert!(matches!(error, MatterFieldError::InvalidSubstrate(_)));

    let mut run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);
    run.source_surface.surface_id = "mesh.planarian_ap.damaged_surface".to_owned();
    let error = run.validate().expect_err("bad source surface rejects");

    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut run = planarian_run(PlanarianBioelectricScenarioKind::Baseline);
    run.substrate.nodes[0].barycentric = [0.75, 0.75, 0.0];
    let error = run.validate().expect_err("bad node anchor rejects");

    assert!(matches!(error, MatterFieldError::InvalidSubstrate(_)));
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
        BioelectricConductanceEdge::from_substrate_neighbors(substrate, 0.18, 0.04, Some(gate))
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

fn planarian_run(kind: PlanarianBioelectricScenarioKind) -> PlanarianBioelectricScenarioRun {
    PlanarianBioelectricScenarioRun::build(kind, PlanarianBioelectricPresetConfig::default())
        .expect("planarian scenario validates")
}

fn average_readout(
    frame: &crate::BioelectricCircuitDebugFrame,
    layer_id: &str,
    node_indices: &[usize],
) -> f32 {
    let layer = frame
        .readout_layers
        .iter()
        .find(|layer| layer.layer_id == layer_id)
        .expect("readout layer");
    average_values(&layer.values, node_indices)
}

fn average_voltage(frame: &crate::BioelectricCircuitDebugFrame, node_indices: &[usize]) -> f32 {
    average_values(&frame.voltage_values, node_indices)
}

fn average_memory(frame: &crate::BioelectricCircuitDebugFrame, node_indices: &[usize]) -> f32 {
    average_values(
        frame.memory_values.as_ref().expect("memory values"),
        node_indices,
    )
}

fn average_values(values: &[f32], node_indices: &[usize]) -> f32 {
    let sum = node_indices
        .iter()
        .map(|node_index| values[*node_index])
        .sum::<f32>();
    sum / test_count_to_f32(node_indices.len())
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 1.0e-6);
}

fn test_count_to_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).expect("test count fits f32"))
}

fn outside_nodes(node_count: usize, excluded: &[usize]) -> Vec<usize> {
    (0..node_count)
        .filter(|node_index| !excluded.contains(node_index))
        .collect()
}

fn average_cross_cut_conductance(run: &PlanarianBioelectricScenarioRun, cut_z: f32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0_usize;
    for edge in &run.initial_circuit.conductance_edges {
        let from_z = run
            .axis_map
            .node_normalized_z(edge.from_node)
            .expect("from node has AP coordinate");
        let to_z = run
            .axis_map
            .node_normalized_z(edge.to_node)
            .expect("to node has AP coordinate");
        if (from_z < cut_z && to_z >= cut_z) || (to_z < cut_z && from_z >= cut_z) {
            sum += edge.base_conductance;
            count += 1;
        }
    }
    sum / test_count_to_f32(count)
}

fn posterior_nodes(run: &PlanarianBioelectricScenarioRun) -> Vec<usize> {
    let mut nodes = run.axis_map.nodes_in_region(PlanarianAxisRegion::Tail);
    nodes.extend(
        run.axis_map
            .nodes_in_region(PlanarianAxisRegion::PostpharyngealTrunk),
    );
    nodes
}
