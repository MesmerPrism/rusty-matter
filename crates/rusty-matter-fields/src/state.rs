use rusty_matter_model::Vec3;

use crate::{
    MatterFieldError, SurfaceFieldSubstrate, SURFACE_FIELD_STATE_SCHEMA_ID,
    SURFACE_SCALAR_FIELD_SCHEMA_ID, SURFACE_VECTOR_FIELD_SCHEMA_ID,
};

/// Built-in scalar field semantics for surface-field states.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceScalarFieldKind {
    /// Schematic membrane-potential-like scalar, not quantitative physiology.
    VmemLike,
    /// Schematic wound or cut response signal.
    WoundSignal,
    /// Schematic morphogen or patterning signal.
    Morphogen,
    /// Generic scalar field for non-bioelectric uses.
    Custom,
}

/// Built-in vector field semantics for surface-field states.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceVectorFieldKind {
    /// Schematic tissue or surface polarity.
    Polarity,
    /// Generic vector field for non-bioelectric uses.
    Custom,
}

/// Scalar values over every node of a surface-field substrate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceScalarField {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable field identifier.
    pub field_id: String,
    /// Field semantic.
    pub kind: SurfaceScalarFieldKind,
    /// One scalar value per substrate node.
    pub values: Vec<f32>,
}

impl SurfaceScalarField {
    /// Creates a scalar field.
    #[must_use]
    pub fn new(
        field_id: impl Into<String>,
        kind: SurfaceScalarFieldKind,
        values: Vec<f32>,
    ) -> Self {
        Self {
            schema_id: SURFACE_SCALAR_FIELD_SCHEMA_ID.to_owned(),
            field_id: field_id.into(),
            kind,
            values,
        }
    }

    /// Creates a constant scalar field.
    #[must_use]
    pub fn constant(
        field_id: impl Into<String>,
        kind: SurfaceScalarFieldKind,
        node_count: usize,
        value: f32,
    ) -> Self {
        Self::new(field_id, kind, vec![value; node_count])
    }

    /// Returns the scalar value range.
    #[must_use]
    pub fn value_range(&self) -> Option<(f32, f32)> {
        let mut values = self.values.iter().copied();
        let first = values.next()?;
        let mut min_value = first;
        let mut max_value = first;
        for value in values {
            min_value = min_value.min(value);
            max_value = max_value.max(value);
        }
        Some((min_value, max_value))
    }

    /// Validates the scalar field against a substrate node count.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, ID, count, or values are
    /// invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_SCALAR_FIELD_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_SCALAR_FIELD_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.field_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyFieldId);
        }
        if self.values.len() != node_count {
            return Err(MatterFieldError::NodeCountMismatch {
                expected: node_count,
                actual: self.values.len(),
            });
        }
        for (index, value) in self.values.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(MatterFieldError::NonFiniteScalar {
                    field_id: self.field_id.clone(),
                    index,
                });
            }
        }
        Ok(())
    }
}

/// Vector values over every node of a surface-field substrate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceVectorField {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable field identifier.
    pub field_id: String,
    /// Field semantic.
    pub kind: SurfaceVectorFieldKind,
    /// One vector value per substrate node.
    pub vectors: Vec<Vec3>,
}

impl SurfaceVectorField {
    /// Creates a vector field.
    #[must_use]
    pub fn new(
        field_id: impl Into<String>,
        kind: SurfaceVectorFieldKind,
        vectors: Vec<Vec3>,
    ) -> Self {
        Self {
            schema_id: SURFACE_VECTOR_FIELD_SCHEMA_ID.to_owned(),
            field_id: field_id.into(),
            kind,
            vectors,
        }
    }

    /// Creates a constant vector field.
    #[must_use]
    pub fn constant(
        field_id: impl Into<String>,
        kind: SurfaceVectorFieldKind,
        node_count: usize,
        vector: Vec3,
    ) -> Self {
        Self::new(field_id, kind, vec![vector; node_count])
    }

    /// Returns the maximum vector length.
    #[must_use]
    pub fn max_vector_length(&self) -> Option<f32> {
        self.vectors
            .iter()
            .copied()
            .map(Vec3::length)
            .max_by(f32::total_cmp)
    }

    /// Validates the vector field against a substrate node count.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, ID, count, or vectors are
    /// invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_VECTOR_FIELD_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_VECTOR_FIELD_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.field_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyFieldId);
        }
        if self.vectors.len() != node_count {
            return Err(MatterFieldError::NodeCountMismatch {
                expected: node_count,
                actual: self.vectors.len(),
            });
        }
        for (index, vector) in self.vectors.iter().copied().enumerate() {
            if !vector.is_finite() {
                return Err(MatterFieldError::NonFiniteVector {
                    field_id: self.field_id.clone(),
                    index,
                });
            }
        }
        Ok(())
    }
}

/// Full surface-field state bound to one substrate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldState {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable state identifier.
    pub state_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Expected node count for all field buffers.
    pub node_count: usize,
    /// State time in seconds.
    pub time_seconds: f32,
    /// Scalar field buffers.
    pub scalar_fields: Vec<SurfaceScalarField>,
    /// Vector field buffers.
    pub vector_fields: Vec<SurfaceVectorField>,
}

impl SurfaceFieldState {
    /// Creates and validates a surface-field state for a substrate.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the substrate or fields are invalid.
    pub fn new(
        state_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        scalar_fields: Vec<SurfaceScalarField>,
        vector_fields: Vec<SurfaceVectorField>,
    ) -> Result<Self, MatterFieldError> {
        substrate.validate()?;
        let state = Self {
            schema_id: SURFACE_FIELD_STATE_SCHEMA_ID.to_owned(),
            state_id: state_id.into(),
            substrate_id: substrate.substrate_id.clone(),
            node_count: substrate.node_count(),
            time_seconds: 0.0,
            scalar_fields,
            vector_fields,
        };
        state.validate()?;
        Ok(state)
    }

    /// Returns a scalar field by ID.
    #[must_use]
    pub fn scalar_field(&self, field_id: &str) -> Option<&SurfaceScalarField> {
        self.scalar_fields
            .iter()
            .find(|field| field.field_id == field_id)
    }

    /// Returns a vector field by ID.
    #[must_use]
    pub fn vector_field(&self, field_id: &str) -> Option<&SurfaceVectorField> {
        self.vector_fields
            .iter()
            .find(|field| field.field_id == field_id)
    }

    /// Validates the state contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata or fields are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_STATE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_STATE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.state_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyStateId);
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.node_count == 0 {
            return Err(MatterFieldError::InvalidField(
                "state node count must be non-zero",
            ));
        }
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(MatterFieldError::InvalidField(
                "state time must be finite and non-negative",
            ));
        }
        if self.scalar_fields.is_empty() && self.vector_fields.is_empty() {
            return Err(MatterFieldError::InvalidField(
                "state must contain at least one field",
            ));
        }
        let mut field_ids = Vec::with_capacity(self.scalar_fields.len() + self.vector_fields.len());
        for field in &self.scalar_fields {
            field.validate(self.node_count)?;
            push_unique_field_id(&mut field_ids, &field.field_id)?;
        }
        for field in &self.vector_fields {
            field.validate(self.node_count)?;
            push_unique_field_id(&mut field_ids, &field.field_id)?;
        }
        Ok(())
    }
}

fn push_unique_field_id(
    field_ids: &mut Vec<String>,
    field_id: &str,
) -> Result<(), MatterFieldError> {
    if field_ids.iter().any(|existing| existing == field_id) {
        Err(MatterFieldError::DuplicateFieldId {
            field_id: field_id.to_owned(),
        })
    } else {
        field_ids.push(field_id.to_owned());
        Ok(())
    }
}
