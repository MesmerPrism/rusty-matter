//! Cross-lane hand rig and CPU-skinning conformance contracts.

use crate::{
    HandJointFrame, HandRigCapture, HandValidationMeshFrame, MatterMeshError,
    HAND_JOINT_FRAME_SCHEMA_ID, HAND_RIG_CAPTURE_SCHEMA_ID,
};

/// Schema id for a provider-bound Matter hand substrate payload.
pub const HAND_SUBSTRATE_SCHEMA_ID: &str = "rusty.matter.hand.substrate.v1";
/// Lattice hand frame schema accepted by this conformance contract.
pub const LATTICE_HAND_JOINT_FRAME_SCHEMA_ID: &str = "rusty.lattice.hand_joint_frame.v1";

/// Provider-bound rig and joint frame used by Matter's deterministic skinning oracle.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[derive(Clone, Debug, PartialEq)]
pub struct HandSubstrateConformance {
    /// Schema id.
    pub schema: String,
    /// Stable conformance id.
    pub conformance_id: String,
    /// Provider identity that must agree with both Matter payloads.
    pub provider_id: String,
    /// Lattice provider-neutral frame id associated with this Matter frame.
    pub lattice_frame_id: String,
    /// Lattice frame schema id.
    pub lattice_frame_schema_id: String,
    /// Coordinate-basis label preserved from Lattice.
    pub coordinate_basis: String,
    /// Neutral bind mesh, hierarchy, and skinning weights.
    pub rig: HandRigCapture,
    /// Neutral joint poses used by the CPU reference skinning path.
    pub joint_frame: HandJointFrame,
}

impl HandSubstrateConformance {
    /// Validate ownership, identity, rig shape, and frame compatibility.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema != HAND_SUBSTRATE_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: HAND_SUBSTRATE_SCHEMA_ID,
                actual: self.schema.clone(),
            });
        }
        if self.conformance_id.trim().is_empty()
            || self.provider_id.trim().is_empty()
            || self.lattice_frame_id.trim().is_empty()
        {
            return Err(MatterMeshError::InvalidHandPayload(
                "conformance, provider, and Lattice frame ids must be non-empty",
            ));
        }
        if self.lattice_frame_schema_id != LATTICE_HAND_JOINT_FRAME_SCHEMA_ID {
            return Err(MatterMeshError::InvalidHandPayload(
                "Lattice hand frame schema must match the accepted contract",
            ));
        }
        if !matches!(
            self.coordinate_basis.as_str(),
            "right_handed_y_up_negative_z_forward" | "right_handed_y_up_positive_z_forward"
        ) {
            return Err(MatterMeshError::InvalidHandPayload(
                "coordinate basis must be an accepted provider-neutral basis",
            ));
        }
        if self.rig.schema_id != HAND_RIG_CAPTURE_SCHEMA_ID
            || self.joint_frame.schema_id != HAND_JOINT_FRAME_SCHEMA_ID
        {
            return Err(MatterMeshError::InvalidHandPayload(
                "rig and frame schemas must match Matter hand contracts",
            ));
        }
        self.rig.validate()?;
        self.joint_frame.validate()?;
        if self.rig.source != self.provider_id || self.joint_frame.source != self.provider_id {
            return Err(MatterMeshError::InvalidHandPayload(
                "provider identity must match rig and joint frame source",
            ));
        }
        if self.rig.handedness != self.joint_frame.handedness
            || self.rig.reference_space != self.joint_frame.reference_space
            || self.rig.joint_count() != self.joint_frame.poses.len()
        {
            return Err(MatterMeshError::InvalidHandPayload(
                "rig and joint frame handedness, reference space, and joint count must match",
            ));
        }
        Ok(())
    }

    /// Run the Matter CPU skinning oracle and return a validation frame.
    pub fn skin_cpu(&self) -> Result<HandValidationMeshFrame, MatterMeshError> {
        self.validate()?;
        self.rig.skin_to_validation_frame(
            &self.joint_frame,
            format!("{}.cpu-skinned", self.conformance_id),
        )
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    const VALID: &str = include_str!("../../../fixtures/hand/hand-substrate-conformance.json");
    const PROVIDER_DAMAGE: &str =
        include_str!("../../../fixtures/damaged/hand-substrate-provider-mixup.json");
    const RIG_DAMAGE: &str =
        include_str!("../../../fixtures/damaged/hand-substrate-invalid-rig.json");

    #[test]
    fn conformance_fixture_skins_deterministically() {
        let substrate: HandSubstrateConformance = serde_json::from_str(VALID).unwrap();
        substrate.validate().unwrap();
        let first = substrate.skin_cpu().unwrap();
        let second = substrate.skin_cpu().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.surface.vertex_count(), 4);
        assert_eq!(first.surface.triangle_count(), 2);
    }

    #[test]
    fn provider_mixup_and_invalid_rig_fail_closed() {
        let valid: HandSubstrateConformance = serde_json::from_str(VALID).unwrap();
        let provider_damage: serde_json::Value = serde_json::from_str(PROVIDER_DAMAGE).unwrap();
        let rig_damage: serde_json::Value = serde_json::from_str(RIG_DAMAGE).unwrap();

        let mut provider_mixup = valid.clone();
        provider_mixup.joint_frame.source = provider_damage["joint_frame_source"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(provider_mixup.validate().is_err());

        let mut invalid_rig = valid;
        invalid_rig.rig.vertex_joint_weights[0][0] =
            rig_damage["first_weight"].as_f64().unwrap() as f32;
        assert!(invalid_rig.validate().is_err());
    }

    #[test]
    fn backend_fields_are_rejected() {
        let damaged = VALID.replace(
            "\"provider_id\": \"generic-tracked-hand-provider\"",
            "\"provider_id\": \"generic-tracked-hand-provider\", \"gpu_buffer\": 9",
        );
        assert!(serde_json::from_str::<HandSubstrateConformance>(&damaged).is_err());
    }
}
