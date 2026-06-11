use rusty_matter_adf::{
    build_adf_from_sdf_grid_report, AdaptiveDistanceField, AdfBuildConfig, AdfBuildReport,
};
use rusty_matter_sdf::PackedSdfGrid;

use crate::summary::AdfFixtureSummary;

#[derive(Clone, Copy)]
pub(crate) struct AdfFixtureCase {
    pub(crate) fixture_id: &'static str,
    pub(crate) field_path: &'static str,
    pub(crate) summary_path: &'static str,
    pub(crate) config: AdfBuildConfig,
}

pub(crate) fn adf_fixture_case_for_grid(grid: &PackedSdfGrid) -> Option<AdfFixtureCase> {
    match grid.grid_id.as_str() {
        "sdf.mesh.unit_triangle" => Some(AdfFixtureCase {
            fixture_id: "fixture.adf.unit_triangle.v1",
            field_path: "fixtures/adf/unit-triangle-adaptive-field.json",
            summary_path: "fixtures/adf/unit-triangle-adf-summary.json",
            config: AdfBuildConfig {
                max_depth: 3,
                max_cells: 1_024,
                error_tolerance: 0.025,
            },
        }),
        _ => None,
    }
}

pub(crate) fn build_adf_fixture(
    grid: &PackedSdfGrid,
    case: AdfFixtureCase,
) -> Result<AdfBuildReport, rusty_matter_adf::AdfError> {
    build_adf_from_sdf_grid_report(grid, case.config)
}

pub(crate) fn summarize_adf_fixture(
    fixture_id: impl Into<String>,
    field: &AdaptiveDistanceField,
    report: &AdfBuildReport,
) -> AdfFixtureSummary {
    let min_cell_distance = field
        .cells
        .iter()
        .map(|cell| cell.min_distance)
        .reduce(f32::min)
        .expect("validated ADF field has cells");
    let max_cell_distance = field
        .cells
        .iter()
        .map(|cell| cell.max_distance)
        .reduce(f32::max)
        .expect("validated ADF field has cells");

    AdfFixtureSummary {
        schema_id: "rusty.matter.fixture.adf_summary.v1".to_owned(),
        fixture_id: fixture_id.into(),
        field_id: field.field_id.clone(),
        source_grid_id: field.source_grid_id.clone(),
        root_origin: field.origin,
        root_extent: field.extent,
        max_depth: field.max_depth,
        source_sample_count: report.diagnostics.source_sample_count,
        cell_count: report.diagnostics.cell_count,
        split_count: report.diagnostics.split_count,
        max_level: report.diagnostics.max_level,
        min_cell_distance,
        max_cell_distance,
    }
}
