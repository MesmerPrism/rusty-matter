use rusty_matter_model::Vec3;

use crate::SdfError;

/// Schema ID for packed SDF grids.
pub const PACKED_SDF_GRID_SCHEMA_ID: &str = "rusty.matter.sdf.packed_grid.v1";

/// Packed SDF grid.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PackedSdfGrid {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable grid identifier.
    pub grid_id: String,
    /// Origin of grid cell `[0, 0, 0]`.
    pub origin: Vec3,
    /// Voxel size in mesh units.
    pub voxel_size: f32,
    /// Grid dimensions `[x, y, z]`.
    pub dimensions: [u32; 3],
    /// Packed distances in x-fastest order.
    pub distances: Vec<f32>,
}

impl PackedSdfGrid {
    /// Creates a packed grid.
    #[must_use]
    pub fn new(
        grid_id: impl Into<String>,
        origin: Vec3,
        voxel_size: f32,
        dimensions: [u32; 3],
        distances: Vec<f32>,
    ) -> Self {
        Self {
            schema_id: PACKED_SDF_GRID_SCHEMA_ID.to_owned(),
            grid_id: grid_id.into(),
            origin,
            voxel_size,
            dimensions,
            distances,
        }
    }

    /// Validates grid metadata and packed samples.
    ///
    /// # Errors
    ///
    /// Returns [`SdfError`] when metadata is invalid or sample count does not
    /// match dimensions.
    pub fn validate(&self) -> Result<(), SdfError> {
        if self.schema_id != PACKED_SDF_GRID_SCHEMA_ID {
            return Err(SdfError::UnexpectedSchema {
                expected: PACKED_SDF_GRID_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.grid_id.trim().is_empty() {
            return Err(SdfError::EmptyGridId);
        }
        if !self.origin.is_finite() {
            return Err(SdfError::NonFiniteOrigin);
        }
        if !self.voxel_size.is_finite() || self.voxel_size <= 0.0 {
            return Err(SdfError::InvalidVoxelSize(self.voxel_size));
        }
        let expected = voxel_count(self.dimensions)?;
        if self.distances.len() != expected {
            return Err(SdfError::DistanceCountMismatch {
                expected,
                actual: self.distances.len(),
            });
        }
        for (index, distance) in self.distances.iter().copied().enumerate() {
            if !distance.is_finite() {
                return Err(SdfError::NonFiniteDistance { index });
            }
        }
        Ok(())
    }

    /// Returns the packed index for a grid coordinate.
    #[must_use]
    pub fn packed_index(&self, x: u32, y: u32, z: u32) -> Option<usize> {
        let [width, height, depth] = self.dimensions;
        if x >= width || y >= height || z >= depth {
            return None;
        }
        let width = usize::try_from(width).ok()?;
        let height = usize::try_from(height).ok()?;
        let x = usize::try_from(x).ok()?;
        let y = usize::try_from(y).ok()?;
        let z = usize::try_from(z).ok()?;
        z.checked_mul(height)?
            .checked_add(y)?
            .checked_mul(width)?
            .checked_add(x)
    }

    /// Returns a distance at a grid coordinate.
    #[must_use]
    pub fn distance_at(&self, x: u32, y: u32, z: u32) -> Option<f32> {
        self.packed_index(x, y, z)
            .and_then(|index| self.distances.get(index).copied())
    }

    /// Samples the nearest grid cell.
    #[must_use]
    pub fn sample_nearest(&self, point: Vec3) -> Option<SdfSample> {
        if !point.is_finite() {
            return None;
        }
        let local = (point - self.origin) / self.voxel_size - Vec3::new(0.5, 0.5, 0.5);
        let x = round_to_u32(local.x)?;
        let y = round_to_u32(local.y)?;
        let z = round_to_u32(local.z)?;
        let distance = self.distance_at(x, y, z)?;
        Some(SdfSample {
            point,
            distance,
            cell: [x, y, z],
        })
    }

    /// Returns the number of grid samples.
    #[must_use]
    pub fn sample_count(&self) -> usize {
        self.distances.len()
    }
}

/// SDF sample result.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdfSample {
    /// Sample point.
    pub point: Vec3,
    /// Signed or unsigned distance.
    pub distance: f32,
    /// Nearest grid cell.
    pub cell: [u32; 3],
}

pub(crate) fn voxel_count(dimensions: [u32; 3]) -> Result<usize, SdfError> {
    let [width, height, depth] = dimensions;
    if width == 0 || height == 0 || depth == 0 {
        return Err(SdfError::ZeroDimension);
    }
    let width = usize::try_from(width).map_err(|_| SdfError::VoxelCountOverflow)?;
    let height = usize::try_from(height).map_err(|_| SdfError::VoxelCountOverflow)?;
    let depth = usize::try_from(depth).map_err(|_| SdfError::VoxelCountOverflow)?;
    width
        .checked_mul(height)
        .and_then(|value| value.checked_mul(depth))
        .ok_or(SdfError::VoxelCountOverflow)
}

fn round_to_u32(value: f32) -> Option<u32> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let rounded = value.round();
    if rounded > u32::MAX as f32 {
        return None;
    }
    Some(rounded as u32)
}
