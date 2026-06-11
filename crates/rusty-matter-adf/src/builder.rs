use rusty_matter_model::Vec3;
use rusty_matter_sdf::PackedSdfGrid;
use std::ops::Range;

use crate::{AdaptiveDistanceField, AdfBuildConfig, AdfCell, AdfError};

/// ADF build diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AdfBuildDiagnostics {
    /// Number of samples in the source dense SDF grid.
    pub source_sample_count: usize,
    /// Number of emitted leaf cells.
    pub cell_count: usize,
    /// Number of split internal cells.
    pub split_count: usize,
    /// Maximum emitted leaf-cell level.
    pub max_level: u32,
}

/// ADF build output with diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct AdfBuildReport {
    /// Built adaptive distance field.
    pub field: AdaptiveDistanceField,
    /// Build diagnostics.
    pub diagnostics: AdfBuildDiagnostics,
}

/// Builds an adaptive distance field from a packed SDF grid.
///
/// # Errors
///
/// Returns [`AdfError`] when the source grid, config, or generated field is
/// invalid.
pub fn build_adf_from_sdf_grid(
    grid: &PackedSdfGrid,
    config: AdfBuildConfig,
) -> Result<AdaptiveDistanceField, AdfError> {
    build_adf_from_sdf_grid_report(grid, config).map(|report| report.field)
}

/// Builds an adaptive distance field from a packed SDF grid and returns
/// diagnostics.
///
/// # Errors
///
/// Returns [`AdfError`] when the source grid, config, or generated field is
/// invalid.
pub fn build_adf_from_sdf_grid_report(
    grid: &PackedSdfGrid,
    config: AdfBuildConfig,
) -> Result<AdfBuildReport, AdfError> {
    grid.validate()?;
    config.validate()?;

    let extent = root_extent(grid)?;
    let mut cells = Vec::new();
    let mut diagnostics = AdfBuildDiagnostics {
        source_sample_count: grid.sample_count(),
        ..AdfBuildDiagnostics::default()
    };
    build_leaf_cells(
        grid,
        config,
        0,
        grid.origin,
        extent,
        &mut cells,
        &mut diagnostics,
    )?;

    let field = AdaptiveDistanceField::new(
        format!("adf.{}", grid.grid_id),
        grid.grid_id.clone(),
        grid.origin,
        extent,
        config.max_depth,
        cells,
    );
    field.validate()?;
    diagnostics.cell_count = field.cell_count();
    Ok(AdfBuildReport { field, diagnostics })
}

fn build_leaf_cells(
    grid: &PackedSdfGrid,
    config: AdfBuildConfig,
    level: u32,
    origin: Vec3,
    extent: f32,
    cells: &mut Vec<AdfCell>,
    diagnostics: &mut AdfBuildDiagnostics,
) -> Result<(), AdfError> {
    let stats = cell_stats(grid, origin, extent);
    let should_split = level < config.max_depth
        && stats.source_sample_count > 1
        && stats.distance_range() > config.error_tolerance;
    if should_split {
        diagnostics.split_count += 1;
        let child_extent = extent * 0.5;
        for z in 0..2 {
            for y in 0..2 {
                for x in 0..2 {
                    let child_origin = origin
                        + Vec3::new(
                            x as f32 * child_extent,
                            y as f32 * child_extent,
                            z as f32 * child_extent,
                        );
                    build_leaf_cells(
                        grid,
                        config,
                        level + 1,
                        child_origin,
                        child_extent,
                        cells,
                        diagnostics,
                    )?;
                }
            }
        }
        return Ok(());
    }

    if cells.len() >= config.max_cells {
        return Err(AdfError::CellBudgetExceeded {
            requested: cells.len().saturating_add(1),
            max: config.max_cells,
        });
    }
    diagnostics.max_level = diagnostics.max_level.max(level);
    cells.push(AdfCell {
        level,
        origin,
        extent,
        center_distance: stats.center_distance,
        min_distance: stats.min_distance,
        max_distance: stats.max_distance,
        source_sample_count: stats.source_sample_count,
    });
    Ok(())
}

fn root_extent(grid: &PackedSdfGrid) -> Result<f32, AdfError> {
    let [width, height, depth] = grid.dimensions;
    let extent = width.max(height).max(depth) as f32 * grid.voxel_size;
    if !extent.is_finite() || extent <= 0.0 {
        return Err(AdfError::InvalidExtent(extent));
    }
    Ok(extent)
}

#[derive(Clone, Copy, Debug)]
struct CellStats {
    center_distance: f32,
    min_distance: f32,
    max_distance: f32,
    source_sample_count: usize,
}

impl CellStats {
    fn distance_range(self) -> f32 {
        self.max_distance - self.min_distance
    }
}

fn cell_stats(grid: &PackedSdfGrid, origin: Vec3, extent: f32) -> CellStats {
    let center = origin + Vec3::new(extent * 0.5, extent * 0.5, extent * 0.5);
    let center_distance = grid
        .sample_nearest_clamped(center)
        .map_or(0.0, |sample| sample.distance);
    let mut min_distance = f32::INFINITY;
    let mut max_distance = f32::NEG_INFINITY;
    let mut source_sample_count = 0usize;

    let [width, height, depth] = grid.dimensions;
    let x_range = cell_axis_range(grid.origin.x, grid.voxel_size, width, origin.x, extent);
    let y_range = cell_axis_range(grid.origin.y, grid.voxel_size, height, origin.y, extent);
    let z_range = cell_axis_range(grid.origin.z, grid.voxel_size, depth, origin.z, extent);
    for z in z_range {
        for y in y_range.clone() {
            for x in x_range.clone() {
                let Some(distance) = grid.distance_at(x, y, z) else {
                    continue;
                };
                source_sample_count += 1;
                min_distance = min_distance.min(distance);
                max_distance = max_distance.max(distance);
            }
        }
    }
    if source_sample_count == 0 {
        min_distance = center_distance;
        max_distance = center_distance;
    }
    CellStats {
        center_distance,
        min_distance,
        max_distance,
        source_sample_count,
    }
}

fn cell_axis_range(
    grid_origin: f32,
    voxel_size: f32,
    dimension: u32,
    cell_origin: f32,
    cell_extent: f32,
) -> Range<u32> {
    if dimension == 0 {
        return 0..0;
    }
    let first_center = (cell_origin - grid_origin) / voxel_size - 0.5;
    let after_last_center = (cell_origin + cell_extent - grid_origin) / voxel_size - 0.5;
    let start = ceil_clamped_to_grid(first_center, dimension);
    let end = ceil_clamped_to_grid(after_last_center, dimension);
    start.min(end)..end
}

fn ceil_clamped_to_grid(value: f32, dimension: u32) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    if value >= dimension as f32 {
        return dimension;
    }
    value.ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_axis_range_matches_half_open_cell_containment() {
        assert_eq!(cell_axis_range(0.0, 1.0, 5, 0.0, 1.0), 0..1);
        assert_eq!(cell_axis_range(0.0, 1.0, 5, 0.5, 2.0), 0..2);
        assert_eq!(cell_axis_range(0.0, 1.0, 5, 1.0, 1.0), 1..2);
        assert_eq!(cell_axis_range(0.0, 1.0, 5, -10.0, 20.0), 0..5);
        assert_eq!(cell_axis_range(0.0, 1.0, 5, 10.0, 1.0), 5..5);
        assert_eq!(cell_axis_range(0.0, 1.0, 5, -1.0, 1.0), 0..0);
    }
}
