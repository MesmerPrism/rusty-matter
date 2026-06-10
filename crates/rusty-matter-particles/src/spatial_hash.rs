use std::collections::BTreeMap;

use rusty_matter_model::Vec3;

use crate::ParticleError;

/// Integer spatial hash cell.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SpatialHashCell {
    /// X cell coordinate.
    pub x: i32,
    /// Y cell coordinate.
    pub y: i32,
    /// Z cell coordinate.
    pub z: i32,
}

/// Deterministic spatial hash grid for particle neighbor candidates.
#[derive(Clone, Debug, Default)]
pub struct SpatialHashGrid {
    cell_size: f32,
    cells: BTreeMap<SpatialHashCell, Vec<usize>>,
}

impl SpatialHashGrid {
    /// Creates a spatial hash.
    #[must_use]
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: BTreeMap::new(),
        }
    }

    /// Rebuilds the hash from particle positions.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when cell size or positions are invalid.
    pub fn build(&mut self, positions: &[Vec3], cell_size: f32) -> Result<(), ParticleError> {
        if !cell_size.is_finite() || cell_size <= 0.0 {
            return Err(ParticleError::InvalidSpatialHashCellSize);
        }
        self.cell_size = cell_size;
        self.cells.clear();
        for (index, position) in positions.iter().copied().enumerate() {
            if !position.is_finite() {
                return Err(ParticleError::NonFinitePosition {
                    particle_id: index.to_string(),
                });
            }
            let cell = cell_for_position(position, self.cell_size);
            self.cells.entry(cell).or_default().push(index);
        }
        Ok(())
    }

    /// Returns candidate particle indices within neighboring hash cells.
    #[must_use]
    pub fn query_radius(&self, position: Vec3, radius: f32) -> Vec<usize> {
        let mut indices = Vec::new();
        self.for_each_candidate(position, radius, |index| indices.push(index));
        indices
    }

    /// Visits candidate particle indices within neighboring hash cells without
    /// allocating a result vector.
    pub fn for_each_candidate(&self, position: Vec3, radius: f32, mut visitor: impl FnMut(usize)) {
        if !position.is_finite() || !radius.is_finite() || radius < 0.0 {
            return;
        }
        let center = cell_for_position(position, self.cell_size);
        let span = (radius / self.cell_size).ceil().max(0.0) as i32;
        for z in (center.z - span)..=(center.z + span) {
            for y in (center.y - span)..=(center.y + span) {
                for x in (center.x - span)..=(center.x + span) {
                    if let Some(cell_indices) = self.cells.get(&SpatialHashCell { x, y, z }) {
                        for index in cell_indices.iter().copied() {
                            visitor(index);
                        }
                    }
                }
            }
        }
    }
}

fn cell_for_position(position: Vec3, cell_size: f32) -> SpatialHashCell {
    SpatialHashCell {
        x: (position.x / cell_size).floor() as i32,
        y: (position.y / cell_size).floor() as i32,
        z: (position.z / cell_size).floor() as i32,
    }
}
