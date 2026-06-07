use crate::SdfError;

/// Sign strategy for mesh-to-SDF conversion.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshSdfSignMode {
    /// Store unsigned nearest-surface distances.
    UnsignedOnly,
    /// Use the nearest triangle normal to choose the sign.
    TriangleNormal,
}

/// CPU mesh-to-SDF builder configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshToSdfConfig {
    /// Grid voxel size in mesh units.
    pub voxel_size: f32,
    /// Number of voxels of padding around mesh bounds.
    pub padding_voxels: u32,
    /// Maximum allowed voxel count.
    pub max_voxels: usize,
    /// Distance sign strategy.
    pub sign_mode: MeshSdfSignMode,
}

impl Default for MeshToSdfConfig {
    fn default() -> Self {
        Self {
            voxel_size: 0.1,
            padding_voxels: 1,
            max_voxels: 1_000_000,
            sign_mode: MeshSdfSignMode::TriangleNormal,
        }
    }
}

impl MeshToSdfConfig {
    /// Validates the builder config.
    ///
    /// # Errors
    ///
    /// Returns [`SdfError`] when voxel size or max voxel budget is invalid.
    pub fn validate(self) -> Result<(), SdfError> {
        if !self.voxel_size.is_finite() || self.voxel_size <= 0.0 {
            return Err(SdfError::InvalidVoxelSize(self.voxel_size));
        }
        if self.max_voxels == 0 {
            return Err(SdfError::InvalidVoxelBudget);
        }
        Ok(())
    }
}
