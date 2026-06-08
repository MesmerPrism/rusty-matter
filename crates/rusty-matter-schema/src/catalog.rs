use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct SchemaCatalog {
    #[serde(rename = "$schema")]
    schema_id: &'static str,
    version: u32,
    entries: Vec<SchemaEntry>,
}

impl SchemaCatalog {
    pub(crate) fn current() -> Self {
        let mesh_fixtures = &[
            "fixtures/mesh/unit-triangle.json",
            "fixtures/mesh/unit-tetrahedron.json",
        ];
        let mesh_surface_fixtures = &["fixtures/mesh/unit-square-surface.json"];
        let mesh_surface_sample_fixtures = &["fixtures/mesh/unit-square-sample-summary.json"];
        let field_contract_fixtures =
            &["fixtures/fields/unit-square-surface-field-run-summary.json"];
        let field_debug_fixtures = &["fixtures/fields/unit-square-surface-field-debug-frame.json"];
        let field_debug_sequence_fixtures =
            &["fixtures/fields/unit-square-surface-field-debug-sequence.json"];
        let bioelectric_circuit_config_fixtures =
            &["fixtures/fields/unit-square-bioelectric-circuit-config.json"];
        let bioelectric_circuit_state_fixtures =
            &["fixtures/fields/unit-square-bioelectric-circuit-state.json"];
        let bioelectric_circuit_diagnostic_fixtures =
            &["fixtures/fields/unit-square-bioelectric-circuit-step-diagnostics.json"];
        let bioelectric_circuit_edit_fixtures =
            &["fixtures/fields/unit-square-bioelectric-circuit-edit.json"];
        let bioelectric_circuit_edit_result_fixtures =
            &["fixtures/fields/unit-square-bioelectric-circuit-edit-result.json"];
        let planarian_bioelectric_scenario_fixtures =
            &["fixtures/fields/planarian-ap-transient-memory-scenario-run.json"];
        let planarian_bioelectric_outcome_fixtures =
            &["fixtures/fields/planarian-ap-transient-memory-outcome-trace.json"];
        let mesh_coordinate_map_fixtures =
            &["fixtures/mesh/unit-square-coordinate-map-summary.json"];
        let mesh_dynamic_collider_fixtures =
            &["fixtures/mesh/unit-square-dynamic-collider-summary.json"];
        let hand_validation_mesh_fixtures =
            &["fixtures/hand/synthetic-hand-validation-mesh-frame.json"];
        let hand_validation_mesh_summary_fixtures =
            &["fixtures/hand/synthetic-hand-validation-mesh-summary.json"];
        let sdf_grid_fixtures = &[
            "fixtures/sdf/unit-triangle-packed-grid.json",
            "fixtures/sdf/unit-tetrahedron-packed-grid.json",
        ];
        let sdf_summary_fixtures = &[
            "fixtures/sdf/unit-triangle-sdf-summary.json",
            "fixtures/sdf/unit-tetrahedron-sdf-summary.json",
        ];
        let damaged_fixtures = &[
            "fixtures/damaged/invalid-coordinate-frame-config.json",
            "fixtures/damaged/invalid-hand-validation-mesh-frame.json",
            "fixtures/damaged/invalid-mesh-index.json",
            "fixtures/damaged/invalid-mesh-surface-index.json",
            "fixtures/damaged/invalid-particle-body.json",
            "fixtures/damaged/invalid-particle-influence.json",
            "fixtures/damaged/invalid-surface-field-perturbation.json",
            "fixtures/damaged/invalid-surface-field-state.json",
            "fixtures/damaged/invalid-voxel-size.json",
            "fixtures/damaged/voxel-budget-overflow.json",
        ];
        let particle_step_fixtures = &[
            "fixtures/particles/interaction-step-summary.json",
            "fixtures/particles/sdf-attraction-step-summary.json",
        ];
        let particle_interaction_fixtures = &["fixtures/particles/interaction-step-summary.json"];
        let particle_render_fixtures = &["fixtures/particles/render-payload-summary.json"];

        Self {
            schema_id: "rusty.matter.schema.catalog.v1",
            version: 1,
            entries: vec![
                entry("rusty.matter.math.vec3.v1", "Vec3", sdf_summary_fixtures),
                entry(
                    "rusty.matter.math.bounds3.v1",
                    "Bounds3",
                    sdf_summary_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.triangle_mesh.v1",
                    "TriangleMeshSnapshot",
                    mesh_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.surface.v1",
                    "TriangleMeshSurface",
                    mesh_surface_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.surface_topology_key.v1",
                    "MeshSurfaceTopologyKey",
                    mesh_surface_sample_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.surface_sample_config.v1",
                    "MeshSurfaceSampleConfig",
                    mesh_surface_sample_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.surface_sample.v1",
                    "MeshSurfaceSample",
                    mesh_surface_sample_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.surface_sample_set.v1",
                    "MeshSurfaceSampleSet",
                    mesh_surface_sample_fixtures,
                ),
                entry(
                    "rusty.matter.fields.surface_node.v1",
                    "SurfaceFieldNode",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.surface_substrate.v1",
                    "SurfaceFieldSubstrate",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.scalar_field.v1",
                    "SurfaceScalarField",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.vector_field.v1",
                    "SurfaceVectorField",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.field_state.v1",
                    "SurfaceFieldState",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.perturbation.v1",
                    "SurfaceFieldPerturbation",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.runtime_config.v1",
                    "SurfaceFieldRuntimeConfig",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.step_diagnostics.v1",
                    "SurfaceFieldStepDiagnostics",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.run_summary.v1",
                    "SurfaceFieldRunSummary",
                    field_contract_fixtures,
                ),
                entry(
                    "rusty.matter.fields.debug_frame.v1",
                    "SurfaceFieldDebugFrame",
                    field_debug_fixtures,
                ),
                entry(
                    "rusty.matter.fields.debug_sequence.v1",
                    "SurfaceFieldDebugFrameSequence",
                    field_debug_sequence_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_voltage_field.v1",
                    "BioelectricVoltageField",
                    bioelectric_circuit_state_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_conductance_edge.v1",
                    "BioelectricConductanceEdge",
                    bioelectric_circuit_state_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_current_term.v1",
                    "BioelectricCurrentTerm",
                    bioelectric_circuit_state_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_readout_layer.v1",
                    "BioelectricReadoutLayer",
                    bioelectric_circuit_state_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_circuit_config.v1",
                    "BioelectricCircuitConfig",
                    bioelectric_circuit_config_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_circuit_state.v1",
                    "BioelectricCircuitState",
                    bioelectric_circuit_state_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_circuit_edit.v1",
                    "BioelectricCircuitEdit",
                    bioelectric_circuit_edit_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_circuit_edit_result.v1",
                    "BioelectricCircuitEditResult",
                    bioelectric_circuit_edit_result_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_step_diagnostics.v1",
                    "BioelectricCircuitStepDiagnostics",
                    bioelectric_circuit_diagnostic_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_circuit_debug_frame.v1",
                    "BioelectricCircuitDebugFrame",
                    planarian_bioelectric_scenario_fixtures,
                ),
                entry(
                    "rusty.matter.fields.bioelectric_circuit_debug_sequence.v1",
                    "BioelectricCircuitDebugSequence",
                    planarian_bioelectric_scenario_fixtures,
                ),
                entry(
                    "rusty.matter.fields.planarian_axis_map.v1",
                    "PlanarianAxisMap",
                    planarian_bioelectric_scenario_fixtures,
                ),
                entry(
                    "rusty.matter.fields.planarian_bioelectric_scenario_run.v1",
                    "PlanarianBioelectricScenarioRun",
                    planarian_bioelectric_scenario_fixtures,
                ),
                entry(
                    "rusty.matter.fields.planarian_bioelectric_outcome_trace.v1",
                    "PlanarianBioelectricOutcomeTrace",
                    planarian_bioelectric_outcome_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.coordinate_map.v1",
                    "MeshCoordinateMap",
                    mesh_coordinate_map_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.coordinate_frame_config.v1",
                    "MeshCoordinateFrameConfig",
                    mesh_coordinate_map_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.coordinate_local_frame.v1",
                    "MeshCoordinateLocalFrame",
                    mesh_coordinate_map_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.coordinate_frame_set.v1",
                    "MeshCoordinateFrameSet",
                    mesh_coordinate_map_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.dynamic_collider_config.v1",
                    "DynamicMeshColliderConfig",
                    mesh_dynamic_collider_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.dynamic_collider_update.v1",
                    "DynamicMeshColliderUpdate",
                    mesh_dynamic_collider_fixtures,
                ),
                entry(
                    "rusty.matter.mesh.dynamic_collider_contact.v1",
                    "DynamicMeshColliderContact",
                    mesh_dynamic_collider_fixtures,
                ),
                entry(
                    "rusty.matter.hand.validation_mesh_frame.v1",
                    "HandValidationMeshFrame",
                    hand_validation_mesh_fixtures,
                ),
                entry(
                    "rusty.matter.sdf.packed_grid.v1",
                    "PackedSdfGrid",
                    sdf_grid_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.sdf_summary.v1",
                    "SdfFixtureSummary",
                    sdf_summary_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.damaged_input_report.v1",
                    "DamagedFixtureReport",
                    damaged_fixtures,
                ),
                entry(
                    "rusty.matter.particle.state.v1",
                    "ParticleState",
                    particle_step_fixtures,
                ),
                entry(
                    "rusty.matter.particle.set.v1",
                    "ParticleSet",
                    particle_step_fixtures,
                ),
                entry(
                    "rusty.matter.particle.sdf_interaction_config.v1",
                    "SdfParticleInteractionConfig",
                    particle_step_fixtures,
                ),
                entry(
                    "rusty.matter.particle.fixed_step_config.v1",
                    "ParticleFixedStepConfig",
                    particle_step_fixtures,
                ),
                entry(
                    "rusty.matter.particle.interactions.v1",
                    "ParticleInteractions",
                    particle_interaction_fixtures,
                ),
                entry(
                    "rusty.matter.particle.influence_point.v1",
                    "ParticleInfluencePoint",
                    particle_interaction_fixtures,
                ),
                entry(
                    "rusty.matter.particle.impulse.v1",
                    "ParticleImpulse",
                    particle_interaction_fixtures,
                ),
                entry(
                    "rusty.matter.particle.interaction_body.v1",
                    "ParticleInteractionBody",
                    particle_interaction_fixtures,
                ),
                entry(
                    "rusty.matter.particle.render_sample.v1",
                    "ParticleRenderSample",
                    particle_render_fixtures,
                ),
                entry(
                    "rusty.matter.particle.render_payload.v1",
                    "ParticleRenderPayload",
                    particle_render_fixtures,
                ),
                entry(
                    "rusty.matter.particle.simulation_diagnostics.v1",
                    "ParticleSimulationDiagnostics",
                    particle_step_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.mesh_surface_sample_summary.v1",
                    "MeshSurfaceSampleSummary",
                    mesh_surface_sample_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.mesh_coordinate_map_summary.v1",
                    "MeshCoordinateMapSummary",
                    mesh_coordinate_map_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.dynamic_collider_summary.v1",
                    "DynamicColliderSummary",
                    mesh_dynamic_collider_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.hand_validation_mesh_summary.v1",
                    "HandValidationMeshSummary",
                    hand_validation_mesh_summary_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.particle_step_summary.v1",
                    "ParticleStepSummary",
                    particle_step_fixtures,
                ),
                entry(
                    "rusty.matter.fixture.particle_render_payload_summary.v1",
                    "ParticleRenderPayloadSummary",
                    particle_render_fixtures,
                ),
            ],
        }
    }
}

#[derive(Serialize)]
struct SchemaEntry {
    schema_id: &'static str,
    rust_type: &'static str,
    fixture_paths: Vec<&'static str>,
    status: &'static str,
}

fn entry(
    schema_id: &'static str,
    rust_type: &'static str,
    fixture_paths: &[&'static str],
) -> SchemaEntry {
    SchemaEntry {
        schema_id,
        rust_type,
        fixture_paths: fixture_paths.to_vec(),
        status: "foundation",
    }
}
