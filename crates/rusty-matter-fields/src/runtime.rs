use crate::{
    MatterFieldError, SurfaceFieldPerturbation, SurfaceFieldRunSummary, SurfaceFieldRuntimeConfig,
    SurfaceFieldState, SurfaceFieldSubstrate,
};

/// Contract-only runtime wrapper for surface-field validation.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldRuntime {
    config: SurfaceFieldRuntimeConfig,
}

impl SurfaceFieldRuntime {
    /// Creates a runtime wrapper from a config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the config is invalid.
    pub fn new(config: SurfaceFieldRuntimeConfig) -> Result<Self, MatterFieldError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Returns the runtime config.
    #[must_use]
    pub fn config(&self) -> &SurfaceFieldRuntimeConfig {
        &self.config
    }

    /// Validates F1 contracts and returns a zero-step summary.
    ///
    /// This does not advance field dynamics; fixed-step updates belong to a
    /// later slice after the contracts and damaged inputs are stable.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the substrate, state, config, or
    /// perturbations are invalid.
    pub fn validate_contracts(
        &self,
        summary_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        state: &SurfaceFieldState,
        perturbations: &[SurfaceFieldPerturbation],
    ) -> Result<SurfaceFieldRunSummary, MatterFieldError> {
        SurfaceFieldRunSummary::from_contracts(
            summary_id,
            substrate,
            state,
            &self.config,
            perturbations,
        )
    }
}
