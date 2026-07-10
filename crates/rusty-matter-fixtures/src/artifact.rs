use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::adf::{adf_fixture_case_for_grid, build_adf_fixture, summarize_adf_fixture};
use crate::damaged::damaged_fixture_reports;
use crate::error::CliError;
use crate::fields::{
    bioelectric_circuit_config, bioelectric_circuit_edit, bioelectric_circuit_edit_result,
    bioelectric_circuit_state, bioelectric_circuit_step_diagnostics,
    planarian_bioelectric_outcome_trace, planarian_bioelectric_outcome_trace_set,
    planarian_bioelectric_scenario_run, planarian_normalized_morphology_metrics,
    planarian_source_dynamics_targets, planarian_species_like_head_taxonomy,
    planarian_xr_display_bridge_fixture, planarian_xr_display_substrate_request,
    planformdb_derived_fixture, surface_field_contract_summary, surface_field_debug_frame,
    surface_field_debug_sequence,
};
use crate::mesh::{
    dynamic_collider_summary, hand_validation_mesh_summary, mesh_coordinate_map_package_summary,
    mesh_coordinate_map_summary, mesh_surface_sample_summary, synthetic_hand_validation_mesh_frame,
    unit_square_surface,
};
use crate::particles::{
    particle_contract_conformance, particle_interaction_step_summary,
    particle_render_payload_summary, particle_sdf_attraction_step_summary,
};
use crate::sdf::{sdf_fixture_cases, summarize_sdf_fixture};
use rusty_matter_sdf::build_sdf_from_mesh;

#[derive(Clone, Debug)]
pub(crate) struct FixtureArtifact {
    relative_path: &'static str,
    json: String,
}

impl FixtureArtifact {
    fn new<T>(relative_path: &'static str, value: &T) -> Result<Self, CliError>
    where
        T: Serialize,
    {
        let json = serde_json::to_string_pretty(value).map_err(CliError::Serialize)?;
        Ok(Self {
            relative_path,
            json,
        })
    }

    pub(crate) fn write(&self, repo_root: &Path) -> Result<(), CliError> {
        let path = repo_root.join(self.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| CliError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&path, format!("{}\n", self.json)).map_err(|source| CliError::Io { path, source })
    }

    pub(crate) fn validate(&self, repo_root: &Path) -> Result<(), CliError> {
        let path = repo_root.join(self.relative_path);
        let existing = fs::read_to_string(&path).map_err(|source| CliError::Io {
            path: path.clone(),
            source,
        })?;
        if existing.trim_end() == self.json.trim_end() {
            Ok(())
        } else {
            Err(CliError::FixtureMismatch {
                path,
                expected: self.json.clone(),
            })
        }
    }
}

pub(crate) fn build_fixture_artifacts() -> Result<Vec<FixtureArtifact>, CliError> {
    let mut artifacts = Vec::new();

    for case in sdf_fixture_cases() {
        let mesh = (case.mesh)();
        let grid = build_sdf_from_mesh(&mesh, case.config).map_err(CliError::Sdf)?;
        let summary = summarize_sdf_fixture(case.fixture_id, &mesh, &grid);

        artifacts.push(FixtureArtifact::new(case.mesh_path, &mesh)?);
        artifacts.push(FixtureArtifact::new(case.grid_path, &grid)?);
        artifacts.push(FixtureArtifact::new(case.summary_path, &summary)?);
        if let Some(adf_case) = adf_fixture_case_for_grid(&grid) {
            let report = build_adf_fixture(&grid, adf_case).map_err(CliError::Adf)?;
            let summary = summarize_adf_fixture(adf_case.fixture_id, &report.field, &report);
            artifacts.push(FixtureArtifact::new(adf_case.field_path, &report.field)?);
            artifacts.push(FixtureArtifact::new(adf_case.summary_path, &summary)?);
        }
    }

    let surface = unit_square_surface();
    artifacts.push(FixtureArtifact::new(
        "fixtures/mesh/unit-square-surface.json",
        &surface,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/mesh/unit-square-sample-summary.json",
        &mesh_surface_sample_summary(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/mesh/unit-square-coordinate-map-summary.json",
        &mesh_coordinate_map_summary(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/mesh/unit-square-coordinate-map-package-summary.json",
        &mesh_coordinate_map_package_summary(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/mesh/unit-square-dynamic-collider-summary.json",
        &dynamic_collider_summary(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-surface-field-run-summary.json",
        &surface_field_contract_summary(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-surface-field-debug-frame.json",
        &surface_field_debug_frame(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-surface-field-debug-sequence.json",
        &surface_field_debug_sequence(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-bioelectric-circuit-config.json",
        &bioelectric_circuit_config()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-bioelectric-circuit-state.json",
        &bioelectric_circuit_state(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-bioelectric-circuit-step-diagnostics.json",
        &bioelectric_circuit_step_diagnostics(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-bioelectric-circuit-edit.json",
        &bioelectric_circuit_edit()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/unit-square-bioelectric-circuit-edit-result.json",
        &bioelectric_circuit_edit_result(&surface)?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-ap-transient-memory-scenario-run.json",
        &planarian_bioelectric_scenario_run()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-ap-transient-memory-outcome-trace.json",
        &planarian_bioelectric_outcome_trace()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-ap-comparison-outcome-trace-set.json",
        &planarian_bioelectric_outcome_trace_set()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-normalized-morphology-metrics.json",
        &planarian_normalized_morphology_metrics()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-species-like-head-taxonomy.json",
        &planarian_species_like_head_taxonomy()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-source-dynamics-targets.json",
        &planarian_source_dynamics_targets()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-xr-neuron-cloud-display-bridge-v0.json",
        &planarian_xr_display_bridge_fixture()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planarian-xr-neuron-cloud-display-substrate-request-v0.json",
        &planarian_xr_display_substrate_request()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/fields/planformdb-derived-v0.json",
        &planformdb_derived_fixture()?,
    )?);

    let hand_frame = synthetic_hand_validation_mesh_frame();
    artifacts.push(FixtureArtifact::new(
        "fixtures/hand/synthetic-hand-validation-mesh-frame.json",
        &hand_frame,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/hand/synthetic-hand-validation-mesh-summary.json",
        &hand_validation_mesh_summary(&hand_frame)?,
    )?);

    for damaged in damaged_fixture_reports()? {
        artifacts.push(FixtureArtifact::new(damaged.path, &damaged.report)?);
    }
    artifacts.push(FixtureArtifact::new(
        "fixtures/particles/contract-conformance.json",
        &particle_contract_conformance()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/particles/sdf-attraction-step-summary.json",
        &particle_sdf_attraction_step_summary()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/particles/interaction-step-summary.json",
        &particle_interaction_step_summary()?,
    )?);
    artifacts.push(FixtureArtifact::new(
        "fixtures/particles/render-payload-summary.json",
        &particle_render_payload_summary()?,
    )?);

    Ok(artifacts)
}
