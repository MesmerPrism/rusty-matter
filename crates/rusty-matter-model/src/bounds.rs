use crate::{MatterModelError, Vec3};

/// Axis-aligned 3D bounds.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3 {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Bounds3 {
    /// Creates validated bounds.
    ///
    /// # Errors
    ///
    /// Returns [`MatterModelError`] when corners are non-finite or inverted.
    pub fn new(min: Vec3, max: Vec3) -> Result<Self, MatterModelError> {
        if !min.is_finite() || !max.is_finite() {
            return Err(MatterModelError::NonFiniteBounds);
        }
        if min.x > max.x || min.y > max.y || min.z > max.z {
            return Err(MatterModelError::InvertedBounds);
        }
        Ok(Self { min, max })
    }

    /// Creates bounds around a point set.
    ///
    /// # Errors
    ///
    /// Returns [`MatterModelError`] when no points are provided or a point is
    /// non-finite.
    pub fn from_points(points: &[Vec3]) -> Result<Self, MatterModelError> {
        let Some(first) = points.first().copied() else {
            return Err(MatterModelError::EmptyPointSet);
        };
        if !first.is_finite() {
            return Err(MatterModelError::NonFinitePoint { index: 0 });
        }

        let mut min = first;
        let mut max = first;
        for (index, point) in points.iter().copied().enumerate().skip(1) {
            if !point.is_finite() {
                return Err(MatterModelError::NonFinitePoint { index });
            }
            min = min.min(point);
            max = max.max(point);
        }

        Self::new(min, max)
    }

    /// Returns the bounds size.
    #[must_use]
    pub fn size(self) -> Vec3 {
        self.max - self.min
    }

    /// Returns padded bounds.
    ///
    /// # Errors
    ///
    /// Returns [`MatterModelError`] when padding is non-finite or negative.
    pub fn padded(self, padding: f32) -> Result<Self, MatterModelError> {
        if !padding.is_finite() || padding < 0.0 {
            return Err(MatterModelError::InvalidPadding);
        }
        let delta = Vec3::new(padding, padding, padding);
        Self::new(self.min - delta, self.max + delta)
    }
}
