use rusty_matter_model::{Bounds3, MatterModelError, TriangleMeshSnapshot, Vec3};

use crate::geometry::{nearest_signed_distance, Triangle};
use crate::grid::voxel_count;
use crate::{MeshToSdfConfig, PackedSdfGrid, SdfError};

/// Dense SDF build diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SdfBuildDiagnostics {
    /// Number of voxels in the generated dense grid.
    pub voxel_count: usize,
    /// Number of triangles prepared from the source mesh.
    pub triangle_count: usize,
    /// Exact triangle distance tests performed by the current brute-force
    /// reference builder.
    pub triangle_tests: usize,
    /// Voxels rejected while building the grid.
    pub rejected_voxels: usize,
}

/// Dense SDF build output with diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SdfBuildReport {
    /// Built packed SDF grid.
    pub grid: PackedSdfGrid,
    /// Build diagnostics.
    pub diagnostics: SdfBuildDiagnostics,
}

/// Builds an SDF grid from a triangle mesh snapshot.
///
/// # Errors
///
/// Returns [`SdfError`] when the mesh, config, or generated grid is invalid.
pub fn build_sdf_from_mesh(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
) -> Result<PackedSdfGrid, SdfError> {
    build_sdf_from_mesh_report(mesh, config).map(|report| report.grid)
}

/// Builds an SDF grid from a triangle mesh snapshot and returns diagnostics.
///
/// # Errors
///
/// Returns [`SdfError`] when the mesh, config, or generated grid is invalid.
pub fn build_sdf_from_mesh_report(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
) -> Result<SdfBuildReport, SdfError> {
    mesh.validate().map_err(SdfError::Model)?;
    config.validate()?;

    let bounds = padded_bounds(mesh.bounds().map_err(SdfError::Model)?, config)?;
    let dimensions = dimensions_for_bounds(bounds, config)?;
    let count = voxel_count(dimensions)?;
    if count > config.max_voxels {
        return Err(SdfError::VoxelBudgetExceeded {
            requested: count,
            max: config.max_voxels,
        });
    }

    let triangles = mesh
        .indices
        .iter()
        .map(|indices| {
            let [a, b, c] = *indices;
            Triangle::new(
                mesh.positions[usize::try_from(a).expect("validated index fits usize")],
                mesh.positions[usize::try_from(b).expect("validated index fits usize")],
                mesh.positions[usize::try_from(c).expect("validated index fits usize")],
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let [width, height, depth] = dimensions;
    let mut distances = Vec::with_capacity(count);
    let mut diagnostics = SdfBuildDiagnostics {
        voxel_count: count,
        triangle_count: triangles.len(),
        ..SdfBuildDiagnostics::default()
    };
    for z in 0..depth {
        for y in 0..height {
            for x in 0..width {
                let point = bounds.min
                    + Vec3::new(
                        (x as f32 + 0.5) * config.voxel_size,
                        (y as f32 + 0.5) * config.voxel_size,
                        (z as f32 + 0.5) * config.voxel_size,
                    );
                let distance = nearest_signed_distance(point, &triangles, config.sign_mode)?;
                diagnostics.triangle_tests =
                    diagnostics.triangle_tests.saturating_add(triangles.len());
                distances.push(distance);
            }
        }
    }

    let grid = PackedSdfGrid::new(
        format!("sdf.{}", mesh.mesh_id),
        bounds.min,
        config.voxel_size,
        dimensions,
        distances,
    );
    grid.validate()?;
    Ok(SdfBuildReport { grid, diagnostics })
}

fn padded_bounds(bounds: Bounds3, config: MeshToSdfConfig) -> Result<Bounds3, SdfError> {
    bounds
        .padded(config.voxel_size * config.padding_voxels as f32)
        .map_err(SdfError::Model)
}

fn dimensions_for_bounds(bounds: Bounds3, config: MeshToSdfConfig) -> Result<[u32; 3], SdfError> {
    let size = bounds.size();
    Ok([
        dimension_for_axis(size.x, config.voxel_size)?,
        dimension_for_axis(size.y, config.voxel_size)?,
        dimension_for_axis(size.z, config.voxel_size)?,
    ])
}

fn dimension_for_axis(size: f32, voxel_size: f32) -> Result<u32, SdfError> {
    if !size.is_finite() || size < 0.0 {
        return Err(SdfError::Model(MatterModelError::InvertedBounds));
    }
    let cells = (size / voxel_size).ceil().max(2.0) as u64;
    u32::try_from(cells)
        .ok()
        .filter(|value| *value > 0)
        .ok_or(SdfError::ZeroDimension)
}
