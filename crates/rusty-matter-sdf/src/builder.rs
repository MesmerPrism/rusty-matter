use rusty_matter_model::{Bounds3, MatterModelError, TriangleMeshSnapshot, Vec3};

use crate::geometry::{nearest_signed_distance, Triangle};
use crate::grid::voxel_count;
use crate::{MeshToSdfConfig, PackedSdfGrid, SdfError};

/// Builds an SDF grid from a triangle mesh snapshot.
///
/// # Errors
///
/// Returns [`SdfError`] when the mesh, config, or generated grid is invalid.
pub fn build_sdf_from_mesh(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
) -> Result<PackedSdfGrid, SdfError> {
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
    Ok(grid)
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
