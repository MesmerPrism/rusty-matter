use rusty_matter_model::Vec3;

use crate::AdfError;

/// Schema ID for adaptive distance fields.
pub const ADAPTIVE_DISTANCE_FIELD_SCHEMA_ID: &str = "rusty.matter.adf.adaptive_distance_field.v1";
/// Default maximum finest-grid cells allowed for an ADF lookup index.
pub const DEFAULT_ADF_INDEX_MAX_GRID_CELLS: usize = 1_000_000;

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
        self.sample_containing_cell(point)
    }

    /// Samples the nearest field cell after clamping `point` to the root cell.
    #[must_use]
    pub fn sample_nearest_clamped(&self, point: Vec3) -> Option<AdfSample> {
        if !point.is_finite() {
            return None;
        }
        self.sample_containing_cell(clamp_point_to_field(point, self.origin, self.extent))
    }

    fn sample_containing_cell(&self, point: Vec3) -> Option<AdfSample> {
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
            cell_center: cell.center(),
            level: cell.level,
        })
    }

    /// Returns the number of leaf cells.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Builds a deterministic CPU lookup index for repeated ADF sampling.
    ///
    /// The index is runtime acceleration data and is intentionally separate
    /// from the serialized field so the field remains a compact leaf-cell
    /// reference artifact.
    ///
    /// # Errors
    ///
    /// Returns [`AdfError`] when the field is invalid or the requested index
    /// would exceed the configured finest-grid cell budget.
    pub fn build_index(
        &self,
        config: AdaptiveDistanceFieldIndexConfig,
    ) -> Result<AdaptiveDistanceFieldIndex, AdfError> {
        AdaptiveDistanceFieldIndex::from_field(self, config)
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
    /// Center of the selected leaf cell.
    pub cell_center: Vec3,
    /// Leaf cell subdivision level.
    pub level: u32,
}

/// CPU ADF index configuration.
///
/// The index maps each finest-grid coordinate at `field.max_depth` to the leaf
/// ADF cell that contains it. This makes repeated particle force sampling O(1)
/// per sample while preserving [`AdaptiveDistanceField`] as the serialized
/// reference field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdaptiveDistanceFieldIndexConfig {
    /// Maximum finest-grid cells allowed in the lookup table.
    pub max_grid_cells: usize,
}

impl Default for AdaptiveDistanceFieldIndexConfig {
    fn default() -> Self {
        Self {
            max_grid_cells: DEFAULT_ADF_INDEX_MAX_GRID_CELLS,
        }
    }
}

impl AdaptiveDistanceFieldIndexConfig {
    /// Validates index settings.
    ///
    /// # Errors
    ///
    /// Returns [`AdfError`] when a setting is invalid.
    pub fn validate(self) -> Result<(), AdfError> {
        if self.max_grid_cells == 0 {
            return Err(AdfError::InvalidIndexGridBudget);
        }
        Ok(())
    }
}

/// Runtime lookup index for repeated ADF sampling.
///
/// This type is deliberately not a schema payload. It is rebuilt from the
/// Matter-owned field when a runtime wants fast repeated sampling, such as a
/// particle force source.
#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveDistanceFieldIndex {
    /// Field ID the index was built for.
    pub field_id: String,
    /// Root cell origin copied from the field.
    pub origin: Vec3,
    /// Root cubic extent copied from the field.
    pub extent: f32,
    /// Maximum subdivision depth copied from the field.
    pub max_depth: u32,
    /// Finest-grid dimension per axis.
    pub grid_dim: u32,
    /// Finest-grid cell extent.
    pub cell_extent: f32,
    /// X-fastest finest-grid lookup table containing leaf-cell indices.
    pub lookup: Vec<usize>,
}

impl AdaptiveDistanceFieldIndex {
    /// Builds an ADF finest-grid lookup index.
    ///
    /// # Errors
    ///
    /// Returns [`AdfError`] when the field is invalid, the index budget is too
    /// small, or the field cells do not cover the finest grid.
    pub fn from_field(
        field: &AdaptiveDistanceField,
        config: AdaptiveDistanceFieldIndexConfig,
    ) -> Result<Self, AdfError> {
        field.validate()?;
        config.validate()?;
        let grid_dim = 1u32
            .checked_shl(field.max_depth)
            .ok_or(AdfError::IndexGridOverflow)?;
        let grid_cell_count = grid_cell_count(grid_dim)?;
        if grid_cell_count > config.max_grid_cells {
            return Err(AdfError::IndexGridBudgetExceeded {
                requested: grid_cell_count,
                max: config.max_grid_cells,
            });
        }
        let cell_extent = field.extent / grid_dim as f32;
        if !cell_extent.is_finite() || cell_extent <= 0.0 {
            return Err(AdfError::InvalidExtent(cell_extent));
        }

        let mut lookup = vec![usize::MAX; grid_cell_count];
        for (cell_index, cell) in field.cells.iter().enumerate() {
            let span = 1u32
                .checked_shl(field.max_depth - cell.level)
                .ok_or(AdfError::IndexGridOverflow)?;
            let start = [
                aligned_grid_coord(cell.origin.x, field.origin.x, cell_extent, grid_dim)
                    .ok_or(AdfError::IndexCellOutOfBounds { index: cell_index })?,
                aligned_grid_coord(cell.origin.y, field.origin.y, cell_extent, grid_dim)
                    .ok_or(AdfError::IndexCellOutOfBounds { index: cell_index })?,
                aligned_grid_coord(cell.origin.z, field.origin.z, cell_extent, grid_dim)
                    .ok_or(AdfError::IndexCellOutOfBounds { index: cell_index })?,
            ];
            let end = [
                start[0]
                    .checked_add(span)
                    .ok_or(AdfError::IndexGridOverflow)?,
                start[1]
                    .checked_add(span)
                    .ok_or(AdfError::IndexGridOverflow)?,
                start[2]
                    .checked_add(span)
                    .ok_or(AdfError::IndexGridOverflow)?,
            ];
            if end[0] > grid_dim
                || end[1] > grid_dim
                || end[2] > grid_dim
                || start[0] >= end[0]
                || start[1] >= end[1]
                || start[2] >= end[2]
            {
                return Err(AdfError::IndexCellOutOfBounds { index: cell_index });
            }
            for z in start[2]..end[2] {
                for y in start[1]..end[1] {
                    for x in start[0]..end[0] {
                        let index = packed_grid_index(x, y, z, grid_dim)?;
                        lookup[index] = cell_index;
                    }
                }
            }
        }
        let missing = lookup.iter().filter(|index| **index == usize::MAX).count();
        if missing > 0 {
            return Err(AdfError::IncompleteIndexGrid { missing });
        }
        Ok(Self {
            field_id: field.field_id.clone(),
            origin: field.origin,
            extent: field.extent,
            max_depth: field.max_depth,
            grid_dim,
            cell_extent,
            lookup,
        })
    }

    /// Samples the indexed leaf cell containing `point`.
    #[must_use]
    pub fn sample_nearest(&self, field: &AdaptiveDistanceField, point: Vec3) -> Option<AdfSample> {
        if !point.is_finite() || !contains_point(point, self.origin, self.extent) {
            return None;
        }
        self.sample_containing_cell(field, point)
    }

    /// Samples the indexed leaf cell after clamping `point` to the root cell.
    #[must_use]
    pub fn sample_nearest_clamped(
        &self,
        field: &AdaptiveDistanceField,
        point: Vec3,
    ) -> Option<AdfSample> {
        if !point.is_finite() {
            return None;
        }
        self.sample_containing_cell(field, clamp_point_to_field(point, self.origin, self.extent))
    }

    /// Returns a normalized finite-difference gradient over indexed ADF
    /// samples, or zero when the local field is flat.
    #[must_use]
    pub fn gradient_nearest(&self, field: &AdaptiveDistanceField, point: Vec3) -> Option<Vec3> {
        let h = self.cell_extent;
        if !h.is_finite() || h <= 0.0 {
            return None;
        }
        let dx = self
            .sample_nearest_clamped(field, point + Vec3::new(h, 0.0, 0.0))?
            .distance
            - self
                .sample_nearest_clamped(field, point - Vec3::new(h, 0.0, 0.0))?
                .distance;
        let dy = self
            .sample_nearest_clamped(field, point + Vec3::new(0.0, h, 0.0))?
            .distance
            - self
                .sample_nearest_clamped(field, point - Vec3::new(0.0, h, 0.0))?
                .distance;
        let dz = self
            .sample_nearest_clamped(field, point + Vec3::new(0.0, 0.0, h))?
            .distance
            - self
                .sample_nearest_clamped(field, point - Vec3::new(0.0, 0.0, h))?
                .distance;
        Some(normalize_or_zero(Vec3::new(dx, dy, dz)))
    }

    fn sample_containing_cell(
        &self,
        field: &AdaptiveDistanceField,
        point: Vec3,
    ) -> Option<AdfSample> {
        if field.field_id != self.field_id {
            return None;
        }
        let [x, y, z] = grid_cell_for_point(point, self.origin, self.cell_extent, self.grid_dim)?;
        let lookup_index = packed_grid_index(x, y, z, self.grid_dim).ok()?;
        let cell_index = self.lookup.get(lookup_index).copied()?;
        let cell = field.cells.get(cell_index)?;
        Some(AdfSample {
            point,
            distance: cell.center_distance,
            cell_index,
            cell_center: cell.center(),
            level: cell.level,
        })
    }
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

fn clamp_point_to_field(point: Vec3, origin: Vec3, extent: f32) -> Vec3 {
    let max = origin + Vec3::new(extent, extent, extent);
    let epsilon = (extent.abs() * 1.0e-6).max(1.0e-6);
    Vec3::new(
        point.x.clamp(origin.x, max.x - epsilon),
        point.y.clamp(origin.y, max.y - epsilon),
        point.z.clamp(origin.z, max.z - epsilon),
    )
}

fn grid_cell_count(grid_dim: u32) -> Result<usize, AdfError> {
    let dim = usize::try_from(grid_dim).map_err(|_| AdfError::IndexGridOverflow)?;
    dim.checked_mul(dim)
        .and_then(|area| area.checked_mul(dim))
        .ok_or(AdfError::IndexGridOverflow)
}

fn packed_grid_index(x: u32, y: u32, z: u32, grid_dim: u32) -> Result<usize, AdfError> {
    if x >= grid_dim || y >= grid_dim || z >= grid_dim {
        return Err(AdfError::IndexGridOverflow);
    }
    let dim = usize::try_from(grid_dim).map_err(|_| AdfError::IndexGridOverflow)?;
    let x = usize::try_from(x).map_err(|_| AdfError::IndexGridOverflow)?;
    let y = usize::try_from(y).map_err(|_| AdfError::IndexGridOverflow)?;
    let z = usize::try_from(z).map_err(|_| AdfError::IndexGridOverflow)?;
    z.checked_mul(dim)
        .and_then(|slice| slice.checked_mul(dim))
        .and_then(|slice| y.checked_mul(dim).and_then(|row| slice.checked_add(row)))
        .and_then(|base| base.checked_add(x))
        .ok_or(AdfError::IndexGridOverflow)
}

fn aligned_grid_coord(value: f32, origin: f32, cell_extent: f32, grid_dim: u32) -> Option<u32> {
    if !value.is_finite() || !origin.is_finite() || !cell_extent.is_finite() || cell_extent <= 0.0 {
        return None;
    }
    let local = (value - origin) / cell_extent;
    if !local.is_finite() {
        return None;
    }
    let rounded = local.round();
    if rounded < 0.0 || rounded > grid_dim as f32 {
        return None;
    }
    Some(rounded as u32)
}

fn grid_cell_for_point(
    point: Vec3,
    origin: Vec3,
    cell_extent: f32,
    grid_dim: u32,
) -> Option<[u32; 3]> {
    Some([
        floor_clamped_to_grid((point.x - origin.x) / cell_extent, grid_dim)?,
        floor_clamped_to_grid((point.y - origin.y) / cell_extent, grid_dim)?,
        floor_clamped_to_grid((point.z - origin.z) / cell_extent, grid_dim)?,
    ])
}

fn floor_clamped_to_grid(value: f32, grid_dim: u32) -> Option<u32> {
    if grid_dim == 0 || !value.is_finite() {
        return None;
    }
    if value <= 0.0 {
        return Some(0);
    }
    if value >= grid_dim as f32 {
        return Some(grid_dim - 1);
    }
    Some(value.floor() as u32)
}

fn normalize_or_zero(vector: Vec3) -> Vec3 {
    if !vector.is_finite() {
        return Vec3::ZERO;
    }
    let length = vector.length();
    if length <= 1.0e-6 {
        Vec3::ZERO
    } else {
        vector / length
    }
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
