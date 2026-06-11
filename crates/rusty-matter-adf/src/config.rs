use crate::AdfError;

/// CPU adaptive-distance-field builder configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdfBuildConfig {
    /// Maximum subdivision depth from the root cell.
    pub max_depth: u32,
    /// Maximum leaf cells allowed in one field.
    pub max_cells: usize,
    /// Maximum accepted distance range within one leaf cell.
    pub error_tolerance: f32,
}

impl Default for AdfBuildConfig {
    fn default() -> Self {
        Self {
            max_depth: 4,
            max_cells: 4_096,
            error_tolerance: 0.025,
        }
    }
}

impl AdfBuildConfig {
    /// Validates builder settings.
    ///
    /// # Errors
    ///
    /// Returns [`AdfError`] when a setting is invalid.
    pub fn validate(self) -> Result<(), AdfError> {
        if self.max_depth > 16 {
            return Err(AdfError::InvalidMaxDepth(self.max_depth));
        }
        if self.max_cells == 0 {
            return Err(AdfError::InvalidCellBudget);
        }
        if !self.error_tolerance.is_finite() || self.error_tolerance < 0.0 {
            return Err(AdfError::InvalidErrorTolerance(self.error_tolerance));
        }
        Ok(())
    }
}
