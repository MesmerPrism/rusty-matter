use rusty_matter_model::Vec3;

use crate::AdfError;

/// Schema ID for adaptive distance fields.
pub const ADAPTIVE_DISTANCE_FIELD_SCHEMA_ID: &str = "rusty.matter.adf.adaptive_distance_field.v1";

/// One leaf cell in an adaptive distance field.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct AdfCell {
    /// Subdivision level from the root cell.
    pub level: u32,
    /// Minimum corner of the cubic cell.
    pub origin: Vec3,
    /// Cubic cell extent.
    pub extent: f32,
    /// Distance sampled at the cell center.
    pub center_distance: f32,
    /// Minimum source SDF sample distance observed in the cell.
    pub min_distance: f32,
    /// Maximum source SDF sample distance observed in the cell.
    pub max_distance: f32,
    /// Source SDF sample count observed in the cell.
    pub source_sample_count: usize,
}

impl AdfCell {
    /// Returns the cell center.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        self.origin + Vec3::new(self.extent * 0.5, self.extent * 0.5, self.extent * 0.5)
    }

    /// Returns whether the cell contains `point`.
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        contains_point(point, self.origin, self.extent)
    }
}

/// Adaptive distance field represented as non-overlapping leaf cells.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveDistanceField {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable ADF identifier.
    pub field_id: String,
    /// Source SDF grid identifier.
    pub source_grid_id: String,
    /// Root cell origin.
    pub origin: Vec3,
    /// Root cubic extent.
    pub extent: f32,
    /// Maximum subdivision depth used by the field.
    pub max_depth: u32,
    /// Leaf cells.
    pub cells: Vec<AdfCell>,
}

impl AdaptiveDistanceField {
    /// Creates an adaptive distance field.
    #[must_use]
    pub fn new(
        field_id: impl Into<String>,
        source_grid_id: impl Into<String>,
        origin: Vec3,
        extent: f32,
        max_depth: u32,
        cells: Vec<AdfCell>,
    ) -> Self {
        Self {
            schema_id: ADAPTIVE_DISTANCE_FIELD_SCHEMA_ID.to_owned(),
            field_id: field_id.into(),
            source_grid_id: source_grid_id.into(),
            origin,
            extent,
            max_depth,
            cells,
        }
    }

    /// Validates field metadata and leaf cells.
    ///
    /// # Errors
    ///
    /// Returns [`AdfError`] when metadata or cell data is invalid.
    pub fn validate(&self) -> Result<(), AdfError> {
        if self.schema_id != ADAPTIVE_DISTANCE_FIELD_SCHEMA_ID {
            return Err(AdfError::UnexpectedSchema {
                expected: ADAPTIVE_DISTANCE_FIELD_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.field_id.trim().is_empty() {
            return Err(AdfError::EmptyFieldId);
        }
        if self.source_grid_id.trim().is_empty() {
            return Err(AdfError::EmptySourceGridId);
        }
        if !self.origin.is_finite() {
            return Err(AdfError::NonFiniteOrigin);
        }
        if !self.extent.is_finite() || self.extent <= 0.0 {
            return Err(AdfError::InvalidExtent(self.extent));
        }
        if self.max_depth > 16 {
            return Err(AdfError::InvalidMaxDepth(self.max_depth));
        }
        if self.cells.is_empty() {
            return Err(AdfError::EmptyCells);
        }
        for (index, cell) in self.cells.iter().enumerate() {
            validate_cell(index, cell, self.max_depth)?;
        }
        Ok(())
    }

    /// Samples the leaf cell containing `point`.
    #[must_use]
    pub fn sample_nearest(&self, point: Vec3) -> Option<AdfSample> {
        if !point.is_finite() || !contains_point(point, self.origin, self.extent) {
            return None;
        }
        let (cell_index, cell) = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.contains(point))
            .max_by_key(|(_, cell)| cell.level)?;
        Some(AdfSample {
            point,
            distance: cell.center_distance,
            cell_index,
            level: cell.level,
        })
    }

    /// Returns the number of leaf cells.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

/// ADF sample result.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct AdfSample {
    /// Sample point.
    pub point: Vec3,
    /// Distance stored by the selected leaf cell.
    pub distance: f32,
    /// Leaf cell index.
    pub cell_index: usize,
    /// Leaf cell subdivision level.
    pub level: u32,
}

pub(crate) fn contains_point(point: Vec3, origin: Vec3, extent: f32) -> bool {
    let max = origin + Vec3::new(extent, extent, extent);
    point.x >= origin.x
        && point.y >= origin.y
        && point.z >= origin.z
        && point.x < max.x
        && point.y < max.y
        && point.z < max.z
}

fn validate_cell(index: usize, cell: &AdfCell, max_depth: u32) -> Result<(), AdfError> {
    if cell.level > max_depth {
        return Err(AdfError::CellLevelExceeded {
            index,
            level: cell.level,
            max_depth,
        });
    }
    if !cell.origin.is_finite() {
        return Err(AdfError::NonFiniteCellOrigin { index });
    }
    if !cell.extent.is_finite() || cell.extent <= 0.0 {
        return Err(AdfError::InvalidExtent(cell.extent));
    }
    if !cell.center_distance.is_finite()
        || !cell.min_distance.is_finite()
        || !cell.max_distance.is_finite()
    {
        return Err(AdfError::NonFiniteDistance { index });
    }
    if cell.min_distance > cell.max_distance {
        return Err(AdfError::InvalidDistanceRange { index });
    }
    Ok(())
}
