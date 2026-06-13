use crate::{
    MatterFieldError, PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID,
    PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID, PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID,
};

const PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE: &str = "derived_planformdb_record";
const SOURCE_REVIEWED_DYNAMICS_EVIDENCE_TYPE: &str = "source_reviewed_dynamics_target";
const SPECIES_LIKE_HEAD_TAXONOMY_EVIDENCE_TYPE: &str = "rights_safe_teaching_taxonomy";
const SPECIES_LIKE_HEAD_SOURCE_TARGET_ANCHOR: &str =
    "source:emmons_bell_2015_ijms::target:species_like_head_labels::future_outcome_taxonomy";
const PLANFORMDB_NOTICE_TEXT: &str = "Planform / PlanformDB Notice\n\nSource: Lobo Lab PlanformDB 2.5.0\nSource page: https://lobolab.umbc.edu/planform/download/\n\nThis Rusty Matter fixture is a small transformed subset of PlanformDB metadata. It does not redistribute the raw SQLite database, paper figures, or morphology images.\n\nPlanform and PlanformDB are provided as-is, without any express or implied warranty. The authors are not liable for damages arising from use of this software or database.\n\nPermission is granted to use and redistribute Planform and PlanformDB freely, subject to these restrictions:\n\n1. The origin of the software and database must not be misrepresented.\n2. Works using the software or database require acknowledgment and citation of the Planform publications.\n3. This notice may not be removed or altered from any distribution.\n\nCitation for the database/application:\n\nLobo D, Malone TJ, Levin M. Planform: an application and database of graph-encoded planarian regenerative experiments. Bioinformatics 29(8), 1098-1100, 2013. DOI: 10.1093/bioinformatics/btt088";

/// One source-reviewed checkpoint for a qualitative planarian dynamics target.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSourceDynamicsCheckpoint {
    /// Stable checkpoint identifier.
    pub checkpoint_id: String,
    /// Human-readable source relation.
    pub source_relation: String,
    /// Time or assay anchor from source review, when available.
    pub timing_anchor: String,
    /// Qualitative observation or label carried into Matter planning.
    pub qualitative_observation: String,
    /// Explicit boundary for Matter behavior.
    pub matter_boundary: String,
}

impl PlanarianSourceDynamicsCheckpoint {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.checkpoint_id.trim().is_empty()
            || self.source_relation.trim().is_empty()
            || self.timing_anchor.trim().is_empty()
            || self.qualitative_observation.trim().is_empty()
            || self.matter_boundary.trim().is_empty()
            || !self.matter_boundary.contains("not calibrated")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics checkpoints must be populated and non-calibrated",
            ));
        }
        Ok(())
    }
}

/// One source-reviewed dynamics target and its allowed Matter links.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSourceDynamicsTarget {
    /// Stable implementation target identifier.
    pub target_id: String,
    /// Source identifiers backing the target.
    pub source_ids: Vec<String>,
    /// Source-target anchor used by Bioelectricity planning.
    pub source_target_anchor: String,
    /// Current implementation status.
    pub source_target_status: String,
    /// Qualitative role for Matter dynamics or annotation.
    pub dynamics_role: String,
    /// Matter scenario identifiers that may reference this target.
    pub matter_scenario_ids: Vec<String>,
    /// PlanformDB-derived record IDs linked to this target, if any.
    pub planformdb_record_ids: Vec<String>,
    /// Allowed uses of this source target.
    pub allowed_uses: Vec<String>,
    /// Explicitly blocked uses.
    pub blocked_uses: Vec<String>,
    /// Source-reviewed checkpoints carried by this target.
    pub checkpoints: Vec<PlanarianSourceDynamicsCheckpoint>,
}

impl PlanarianSourceDynamicsTarget {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.target_id.trim().is_empty()
            || self.source_ids.is_empty()
            || self.source_target_anchor.trim().is_empty()
            || self.source_target_status.trim().is_empty()
            || self.dynamics_role.trim().is_empty()
            || self.allowed_uses.is_empty()
            || self.blocked_uses.is_empty()
            || self.checkpoints.is_empty()
            || self
                .source_ids
                .iter()
                .any(|source_id| source_id.trim().is_empty())
            || self
                .allowed_uses
                .iter()
                .any(|allowed_use| allowed_use.trim().is_empty())
            || self
                .blocked_uses
                .iter()
                .any(|blocked_use| blocked_use.trim().is_empty())
            || self
                .matter_scenario_ids
                .iter()
                .any(|scenario_id| scenario_id.trim().is_empty())
            || self
                .planformdb_record_ids
                .iter()
                .any(|record_id| record_id.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics target metadata must be populated",
            ));
        }
        if !self
            .source_target_anchor
            .contains(&format!("target:{}", self.target_id))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics anchor must reference the target ID",
            ));
        }
        if !self
            .blocked_uses
            .iter()
            .any(|blocked_use| blocked_use.contains("calibrated physiology"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics targets must block calibrated physiology claims",
            ));
        }
        if self
            .source_ids
            .iter()
            .any(|source_id| source_id == "planformdb_250")
            && self.planformdb_record_ids.is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB-backed dynamics targets must preserve derived record IDs",
            ));
        }
        if self
            .source_ids
            .iter()
            .any(|source_id| source_id == "planformdb_250")
            && self
                .planformdb_record_ids
                .iter()
                .any(|record_id| !record_id.starts_with("planformdb:experiment:"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB-backed dynamics targets must use PlanformDB record IDs",
            ));
        }
        let mut seen_checkpoint_ids = Vec::<&str>::with_capacity(self.checkpoints.len());
        for checkpoint in &self.checkpoints {
            checkpoint.validate()?;
            if seen_checkpoint_ids.contains(&checkpoint.checkpoint_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian source dynamics target must not repeat checkpoint IDs",
                ));
            }
            seen_checkpoint_ids.push(checkpoint.checkpoint_id.as_str());
        }
        Ok(())
    }
}

/// Matter-owned fixture for source-reviewed planarian dynamics targets.
///
/// This fixture is annotation and validation data. It does not change the
/// synthetic educational voltage, conductance, memory, or readout stepping
/// behavior.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSourceDynamicsTargetFixture {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable fixture identifier.
    pub fixture_id: String,
    /// Fixture schema version.
    pub schema_version: u32,
    /// Evidence type; must be `source_reviewed_dynamics_target`.
    pub evidence_type: String,
    /// Human-readable scope.
    pub scope: String,
    /// Overall non-calibration policy.
    pub source_policy: String,
    /// Source-reviewed target rows.
    pub targets: Vec<PlanarianSourceDynamicsTarget>,
}

impl PlanarianSourceDynamicsTargetFixture {
    /// Validates the source dynamics target fixture.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, policy, target identity, or
    /// non-calibration boundaries are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.fixture_id.trim().is_empty()
            || self.schema_version == 0
            || self.evidence_type != SOURCE_REVIEWED_DYNAMICS_EVIDENCE_TYPE
            || self.scope.trim().is_empty()
            || !self.source_policy.contains("not calibrated")
            || self.targets.is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics fixture metadata must be populated and non-calibrated",
            ));
        }
        let mut seen_target_ids = Vec::<&str>::with_capacity(self.targets.len());
        for target in &self.targets {
            target.validate()?;
            if seen_target_ids.contains(&target.target_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian source dynamics fixture must not repeat target IDs",
                ));
            }
            seen_target_ids.push(target.target_id.as_str());
        }
        for required in [
            "ap_transient_memory",
            "gap_block_conductance",
            "head_vs_tail_voltage",
        ] {
            if !seen_target_ids.contains(&required) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian source dynamics fixture is missing a required high-confidence target",
                ));
            }
        }
        Ok(())
    }
}

/// Source database metadata for a curated PlanformDB-derived fixture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbSourceDatabase {
    /// Public source identifier.
    pub id: String,
    /// Source database version.
    pub version: String,
    /// Public download page.
    pub source_url: String,
    /// SHA-256 of the raw source database used for derivation.
    pub raw_sha256: String,
    /// Source SQLite schema version observed during intake.
    pub sqlite_schema_version: u32,
}

impl PlanformDbSourceDatabase {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.source_url.trim().is_empty()
            || self.raw_sha256.trim().is_empty()
            || self.sqlite_schema_version == 0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB source database metadata must be populated",
            ));
        }
        if self.raw_sha256.len() != 64
            || !self.raw_sha256.chars().all(|character| {
                character.is_ascii_digit()
                    || ('a'..='f').contains(&character)
                    || ('A'..='F').contains(&character)
            })
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB source database hash must be SHA-256 hex",
            ));
        }
        Ok(())
    }
}

/// Citation metadata carried with a curated PlanformDB fixture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbCitation {
    /// Stable citation identifier.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// DOI string.
    pub doi: String,
    /// Public URL.
    pub url: String,
}

impl PlanformDbCitation {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.id.trim().is_empty()
            || self.label.trim().is_empty()
            || self.doi.trim().is_empty()
            || self.url.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB citation metadata must be populated",
            ));
        }
        Ok(())
    }
}

/// Selection boundary for a curated PlanformDB-derived fixture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbSelectionPolicy {
    /// Selection date in ISO-8601 calendar form.
    pub selected_on: String,
    /// Short explanation for the selected curated subset.
    pub selection_basis: String,
    /// Explicit non-scope strings.
    pub non_scope: Vec<String>,
}

impl PlanformDbSelectionPolicy {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.selected_on.trim().is_empty()
            || self.selection_basis.trim().is_empty()
            || self.non_scope.is_empty()
            || self.non_scope.iter().any(|item| item.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB selection policy must be populated",
            ));
        }
        if !self
            .non_scope
            .iter()
            .any(|item| item == "Matter runtime dynamics")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB fixture must explicitly exclude runtime dynamics",
            ));
        }
        Ok(())
    }
}

/// Preserved PlanformDB integer source IDs for one derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanformDbSourceIds {
    /// Source publication ID.
    pub publication_id: u32,
    /// Source species ID.
    pub species_id: u32,
    /// Source experiment ID.
    pub experiment_id: u32,
    /// Source manipulation ID.
    pub manipulation_id: u32,
    /// Source result-set ID.
    pub result_set_id: u32,
}

impl PlanformDbSourceIds {
    fn validate(self) -> Result<(), MatterFieldError> {
        if self.publication_id == 0
            || self.species_id == 0
            || self.experiment_id == 0
            || self.manipulation_id == 0
            || self.result_set_id == 0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB source IDs must be positive",
            ));
        }
        Ok(())
    }
}

/// Normalized teaching labels for one PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbNormalizedLabels {
    /// Source publication identifier used by the Bioelectricity target matrix.
    pub publication_source_id: String,
    /// Normalized species label.
    pub species: String,
    /// Normalized manipulation label.
    pub manipulation: String,
    /// Normalized perturbation labels.
    pub perturbations: Vec<String>,
    /// Teaching target linked to this record.
    pub teaching_target: String,
}

impl PlanformDbNormalizedLabels {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.publication_source_id.trim().is_empty()
            || self.species.trim().is_empty()
            || self.manipulation.trim().is_empty()
            || self.teaching_target.trim().is_empty()
            || self.perturbations.is_empty()
            || self
                .perturbations
                .iter()
                .any(|label| label.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB normalized labels must be populated",
            ));
        }
        Ok(())
    }
}

/// Assay context carried by one PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanformDbAssayContext {
    /// Source sample count.
    pub sample_count: u32,
    /// Regeneration period in days.
    pub regeneration_period_days: f32,
}

impl PlanformDbAssayContext {
    fn validate(self) -> Result<(), MatterFieldError> {
        if self.sample_count == 0
            || !self.regeneration_period_days.is_finite()
            || self.regeneration_period_days < 0.0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB assay context must have positive samples and non-negative duration",
            ));
        }
        Ok(())
    }
}

/// One normalized resultant morphology entry from a PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbResultantMorphology {
    /// Source resultant morphology ID.
    pub resultant_morphology_id: u32,
    /// Source morphology ID.
    pub morphology_id: u32,
    /// Normalized outcome label.
    pub normalized_outcome: String,
    /// Source frequency normalized to 0..=1.
    pub frequency: f32,
}

impl PlanformDbResultantMorphology {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.resultant_morphology_id == 0
            || self.morphology_id == 0
            || self.normalized_outcome.trim().is_empty()
            || !self.frequency.is_finite()
            || !(0.0..=1.0).contains(&self.frequency)
        {
            return Err(MatterFieldError::InvalidField(
                "PlanformDB resultant morphology fields must be populated and normalized",
            ));
        }
        Ok(())
    }
}

/// One curated PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbDerivedRecord {
    /// Stable derived record ID.
    pub record_id: String,
    /// Evidence label; must be `derived_planformdb_record`.
    pub evidence_type: String,
    /// Citation/source IDs associated with the record.
    pub source_citation_ids: Vec<String>,
    /// Preserved source integer IDs.
    pub source_ids: PlanformDbSourceIds,
    /// Normalized teaching labels.
    pub normalized_labels: PlanformDbNormalizedLabels,
    /// Source assay context.
    pub assay_context: PlanformDbAssayContext,
    /// Normalized resultant morphology frequencies.
    pub resultant_morphologies: Vec<PlanformDbResultantMorphology>,
    /// Transformation notes and non-calibration boundary.
    pub transform_notes: Vec<String>,
}

impl PlanformDbDerivedRecord {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.record_id
            != format!(
                "planformdb:experiment:{}:resultset:{}",
                self.source_ids.experiment_id, self.source_ids.result_set_id
            )
            || self.evidence_type != PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE
            || self.source_citation_ids.is_empty()
            || self
                .source_citation_ids
                .iter()
                .any(|source_id| source_id.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived record identity and evidence metadata must be populated",
            ));
        }
        if !self
            .source_citation_ids
            .iter()
            .any(|source_id| source_id == "planformdb_250")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived records must retain the PlanformDB source citation",
            ));
        }
        self.source_ids.validate()?;
        self.normalized_labels.validate()?;
        self.assay_context.validate()?;
        if self.resultant_morphologies.is_empty() {
            return Err(MatterFieldError::InvalidField(
                "PlanformDB records require resultant morphology frequencies",
            ));
        }
        let mut frequency_sum = 0.0;
        for morphology in &self.resultant_morphologies {
            morphology.validate()?;
            frequency_sum += morphology.frequency;
        }
        if (frequency_sum - 1.0).abs() > 0.001 {
            return Err(MatterFieldError::InvalidField(
                "PlanformDB resultant morphology frequencies must sum to 1.0",
            ));
        }
        if self.transform_notes.is_empty()
            || self
                .transform_notes
                .iter()
                .any(|note| note.trim().is_empty())
            || !self
                .transform_notes
                .iter()
                .any(|note| note.contains("No calibrated bioelectric physiology"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB transform notes must include the non-calibration boundary",
            ));
        }
        Ok(())
    }
}

/// Curated Matter fixture containing small PlanformDB-derived review records.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbDerivedFixture {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable fixture identifier.
    pub fixture_id: String,
    /// Fixture schema version.
    pub schema_version: u32,
    /// Root evidence type.
    pub evidence_type: String,
    /// Human-readable scope and boundary.
    pub scope: String,
    /// Source database metadata.
    pub source_database: PlanformDbSourceDatabase,
    /// Required notice text that must travel with the fixture.
    pub notice_text: String,
    /// Citations associated with the fixture.
    pub citations: Vec<PlanformDbCitation>,
    /// Selection and non-scope boundary.
    pub selection_policy: PlanformDbSelectionPolicy,
    /// Curated derived records.
    pub records: Vec<PlanformDbDerivedRecord>,
}

impl PlanformDbDerivedFixture {
    /// Validates the fixture contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata, notice text, citations, or
    /// derived records are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.fixture_id.trim().is_empty()
            || self.schema_version == 0
            || self.evidence_type != PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE
            || self.scope.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived fixture metadata must be populated",
            ));
        }
        self.source_database.validate()?;
        self.selection_policy.validate()?;
        validate_planformdb_notice(&self.notice_text)?;
        if self.citations.len() < 2 || self.records.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived fixture requires citations and records",
            ));
        }
        for citation in &self.citations {
            citation.validate()?;
        }
        if !self
            .citations
            .iter()
            .any(|citation| citation.id == "lobo_2013_planform")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived fixture must cite Planform",
            ));
        }
        let mut seen_record_ids = Vec::<&str>::with_capacity(self.records.len());
        for record in &self.records {
            record.validate()?;
            if seen_record_ids.contains(&record.record_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "PlanformDB derived fixture must not repeat record IDs",
                ));
            }
            seen_record_ids.push(record.record_id.as_str());
        }
        Ok(())
    }
}

/// Rights-safe species-like head label entry.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSpeciesLikeHeadLabel {
    /// Stable label identifier.
    pub label_id: String,
    /// Human-readable display label.
    pub display_label: String,
    /// `derived_source_label` or `synthetic_teaching_label`.
    pub label_kind: String,
    /// Source relation or teaching-boundary note.
    pub source_relation: String,
    /// Policy for visual assets associated with this label.
    pub visual_policy: String,
}

impl PlanarianSpeciesLikeHeadLabel {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.label_id.trim().is_empty()
            || self.display_label.trim().is_empty()
            || self.source_relation.trim().is_empty()
            || self.visual_policy.trim().is_empty()
            || !matches!(
                self.label_kind.as_str(),
                "derived_source_label" | "synthetic_teaching_label"
            )
            || !self.visual_policy.contains("generated")
        {
            return Err(MatterFieldError::InvalidField(
                "planarian species-like head labels must be rights-safe generated labels",
            ));
        }
        Ok(())
    }
}

/// Rights-safe categorical taxonomy for species-like head-shape teaching labels.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSpeciesLikeHeadTaxonomy {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable taxonomy identifier.
    pub taxonomy_id: String,
    /// Evidence type for this taxonomy.
    pub evidence_type: String,
    /// Source-target anchor represented by this taxonomy.
    pub source_target_anchor: String,
    /// Source ID.
    pub source_id: String,
    /// Policy against paper-image reuse.
    pub image_policy: String,
    /// Label entries.
    pub labels: Vec<PlanarianSpeciesLikeHeadLabel>,
}

impl PlanarianSpeciesLikeHeadTaxonomy {
    /// Validates the taxonomy contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, policy, source target, or
    /// labels are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.taxonomy_id.trim().is_empty()
            || self.evidence_type != SPECIES_LIKE_HEAD_TAXONOMY_EVIDENCE_TYPE
            || self.source_target_anchor != SPECIES_LIKE_HEAD_SOURCE_TARGET_ANCHOR
            || self.source_id != "emmons_bell_2015_ijms"
            || !self.image_policy.contains("no paper figure reuse")
            || self.labels.is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian species-like head taxonomy metadata must preserve source and image policy",
            ));
        }
        let mut seen_label_ids = Vec::<&str>::with_capacity(self.labels.len());
        for label in &self.labels {
            label.validate()?;
            if seen_label_ids.contains(&label.label_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian species-like head taxonomy must not repeat labels",
                ));
            }
            seen_label_ids.push(label.label_id.as_str());
        }
        if !seen_label_ids.contains(&"unclassified_teaching_abstraction") {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian species-like head taxonomy requires an unclassified teaching label",
            ));
        }
        Ok(())
    }
}

/// Builds the default rights-safe species-like head label taxonomy.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated taxonomy fails validation.
pub fn default_planarian_species_like_head_taxonomy(
) -> Result<PlanarianSpeciesLikeHeadTaxonomy, MatterFieldError> {
    let taxonomy = PlanarianSpeciesLikeHeadTaxonomy {
        schema_id: PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID.to_owned(),
        taxonomy_id: "taxonomy.planarian.species_like_head_labels.v1".to_owned(),
        evidence_type: SPECIES_LIKE_HEAD_TAXONOMY_EVIDENCE_TYPE.to_owned(),
        source_target_anchor: SPECIES_LIKE_HEAD_SOURCE_TARGET_ANCHOR.to_owned(),
        source_id: "emmons_bell_2015_ijms".to_owned(),
        image_policy: "generated symbolic labels only; no paper figure reuse".to_owned(),
        labels: vec![
            head_label(
                "native_gd_like",
                "native G. dorotocephala-like",
                "derived_source_label",
                "control/native or pseudo-native outcome category",
            ),
            head_label(
                "pseudo_dj_like",
                "D. japonica-like",
                "derived_source_label",
                "species-like pseudo morphology category",
            ),
            head_label(
                "pseudo_pf_like",
                "P. felina-like",
                "derived_source_label",
                "species-like pseudo morphology category; source notes incomplete mimicry",
            ),
            head_label(
                "pseudo_sm_like",
                "S. mediterranea-like",
                "derived_source_label",
                "species-like pseudo morphology category",
            ),
            head_label(
                "failed_ap_regeneration",
                "failed AP regeneration",
                "derived_source_label",
                "scored failure class in source frequency panel",
            ),
            head_label(
                "unclassified_teaching_abstraction",
                "unclassified teaching abstraction",
                "synthetic_teaching_label",
                "synthetic or unresolved teaching category",
            ),
        ],
    };
    taxonomy.validate()?;
    Ok(taxonomy)
}

/// Builds the default source-reviewed planarian dynamics target fixture.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated target fixture fails
/// validation.
pub fn default_planarian_source_dynamics_targets(
) -> Result<PlanarianSourceDynamicsTargetFixture, MatterFieldError> {
    let fixture = PlanarianSourceDynamicsTargetFixture {
        schema_id: PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID.to_owned(),
        fixture_id: "fixture.fields.planarian_ap.source_dynamics_targets".to_owned(),
        schema_version: 1,
        evidence_type: SOURCE_REVIEWED_DYNAMICS_EVIDENCE_TYPE.to_owned(),
        scope: "Source-reviewed qualitative planarian dynamics targets for Matter annotation and validation; not runtime calibration.".to_owned(),
        source_policy: "Targets may label synthetic educational scenarios and future derived fixtures; source checkpoints are not calibrated physiology and do not alter Matter stepping.".to_owned(),
        targets: vec![
            source_dynamics_target(
                "ap_transient_memory",
                &["durant_2019_bpj"],
                "source:durant_2019_bpj::target:ap_transient_memory::synthetic_fixture_source_targets_scoped",
                "synthetic_fixture_source_targets_scoped",
                "Early transient depolarization and washout-memory teaching target.",
                &[
                    "bioelectric.planarian_ap.transient_depolarization_memory.synthetic",
                    "bioelectric.planarian_ap.transient_depolarization_no_memory_control.synthetic",
                ],
                &[],
                &[
                    "metadata checkpoint anchors",
                    "memory versus no-memory scenario labeling",
                    "future derived experiment fixture review",
                ],
                &[
                    "calibrated physiology",
                    "mapping source frequencies to stochastic Matter behavior",
                    "millivolt or ion-channel constants",
                ],
                vec![
                    dynamics_checkpoint(
                        "durant_2019_3hpa_window",
                        "Durant 2019 early post-amputation bioelectric window",
                        "3 hpa",
                        "Early depolarization timing is a source checkpoint for transient-memory presets.",
                    ),
                    dynamics_checkpoint(
                        "durant_2019_6hpa_context",
                        "Durant 2019 early AP-polarity context",
                        "6 hpa",
                        "Later early-window context remains annotation until derived experiment data exists.",
                    ),
                    dynamics_checkpoint(
                        "durant_2019_washout_memory",
                        "Durant 2019 washout and later morphology relation",
                        "washout/outcome",
                        "Brief perturbation followed by washout may still label persistent outcome targets.",
                    ),
                ],
            ),
            source_dynamics_target(
                "gap_block_conductance",
                &[
                    "oviedo_2010_devbiol",
                    "emmons_bell_2015_ijms",
                    "planformdb_250",
                    "lobo_2013_planform",
                ],
                "source:oviedo_2010_devbiol;source:emmons_bell_2015_ijms::target:gap_block_conductance::synthetic_fixture_source_targets_scoped",
                "synthetic_fixture_source_targets_scoped",
                "Gap-junction-like coupling reduction, VNC disruption labels, and innexin RNAi labels kept distinct.",
                &["bioelectric.planarian_ap.gap_block.synthetic"],
                &[
                    "planformdb:experiment:415:resultset:467",
                    "planformdb:experiment:416:resultset:468",
                    "planformdb:experiment:417:resultset:469",
                    "planformdb:experiment:418:resultset:470",
                    "planformdb:experiment:419:resultset:471",
                    "planformdb:experiment:441:resultset:493",
                    "planformdb:experiment:442:resultset:494",
                    "planformdb:experiment:443:resultset:495",
                    "planformdb:experiment:444:resultset:496",
                    "planformdb:experiment:446:resultset:498",
                    "planformdb:experiment:447:resultset:499",
                    "planformdb:experiment:448:resultset:500",
                    "planformdb:experiment:449:resultset:501",
                    "planformdb:experiment:450:resultset:502",
                ],
                &[
                    "qualitative conductance-block scenario labels",
                    "PlanformDB-derived phenotype/outcome annotation",
                    "future source-table threshold review",
                ],
                &[
                    "calibrated physiology",
                    "converting PlanformDB frequencies into stochastic simulation",
                    "collapsing octanol, VNC disruption, and innexin RNAi into one mechanism",
                ],
                vec![
                    dynamics_checkpoint(
                        "oviedo_2010_octanol_gap_block",
                        "Oviedo 2010 octanol gap-junction blockade records",
                        "assay/result-set",
                        "Octanol labels support conductance-block metadata, not conductance constants.",
                    ),
                    dynamics_checkpoint(
                        "oviedo_2010_vnc_disruption",
                        "Oviedo 2010 VNC disruption records",
                        "assay/result-set",
                        "VNC disruption remains a separate label from generic coupling reduction.",
                    ),
                    dynamics_checkpoint(
                        "oviedo_2010_innexin_rnai",
                        "Oviedo 2010 innexin RNAi records",
                        "35 day regeneration period",
                        "Innexin labels are evidence annotations, not direct conductance scalars.",
                    ),
                    dynamics_checkpoint(
                        "emmons_2015_species_like_heads",
                        "Emmons-Bell 2015 stochastic species-like head taxonomy",
                        "figure/table target",
                        "Species-like categories support rights-safe labels and future derived mappings.",
                    ),
                ],
            ),
            source_dynamics_target(
                "head_vs_tail_voltage",
                &["beane_2011_chembiol"],
                "source:beane_2011_chembiol::target:head_vs_tail_voltage::active_annotation_metadata",
                "active_annotation_metadata",
                "Voltage/pump perturbation annotation for head-vs-tail identity context.",
                &["bioelectric.planarian_ap.baseline.synthetic"],
                &[],
                &[
                    "normalized voltage unit-policy annotation",
                    "future named pump/channel source review",
                    "Optics display of source-target metadata",
                ],
                &[
                    "calibrated physiology",
                    "H,K-ATPase constant import",
                    "named ion-channel solver behavior",
                    "millivolt fixture without source-value extraction",
                ],
                vec![
                    dynamics_checkpoint(
                        "beane_2011_hk_atpase_annotation",
                        "Beane 2011 H,K-ATPase-mediated membrane-voltage context",
                        "assay/figure target",
                        "Pump/channel language can label source context but cannot set constants yet.",
                    ),
                    dynamics_checkpoint(
                        "beane_2011_head_tail_identity",
                        "Beane 2011 head regeneration identity context",
                        "source text/figure review pending",
                        "Head-vs-tail voltage remains normalized annotation until values are extracted.",
                    ),
                ],
            ),
            source_dynamics_target(
                "persistent_axis_recut_history",
                &["oviedo_2010_devbiol"],
                "source:oviedo_2010_devbiol::target:persistent_axis_recut_history::future_session_trace",
                "future_session_trace",
                "Persistent AP-axis and repeated-regeneration history target for future package/session fixtures.",
                &[],
                &[],
                &[
                    "future experiment/session package target",
                    "recut history annotation",
                    "Manifold audit-surface planning",
                ],
                &[
                    "calibrated physiology",
                    "claiming current Matter scenario reproduces persistent axes",
                    "storing session history inside one static scenario run",
                ],
                vec![dynamics_checkpoint(
                    "oviedo_2010_ectopic_persistent_axis",
                    "Oviedo 2010 ectopic anterior and persistent-axis target",
                    "repeated-regeneration source target",
                    "Persistent history remains future session/package evidence, not current runtime state.",
                )],
            ),
        ],
    };
    fixture.validate()?;
    Ok(fixture)
}

/// Builds the default small PlanformDB-derived fixture.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated fixture fails validation.
pub fn default_planformdb_derived_fixture() -> Result<PlanformDbDerivedFixture, MatterFieldError> {
    let fixture = PlanformDbDerivedFixture {
        schema_id: PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID.to_owned(),
        fixture_id: "planformdb-derived-v0".to_owned(),
        schema_version: 1,
        evidence_type: PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE.to_owned(),
        scope: "Small reviewed PlanformDB-derived metadata fixture for Matter validation; not runtime dynamics.".to_owned(),
        source_database: PlanformDbSourceDatabase {
            id: "planformdb_250".to_owned(),
            version: "2.5.0".to_owned(),
            source_url: "https://lobolab.umbc.edu/planform/download/".to_owned(),
            raw_sha256: "9EFFD13DDB87664B9EF7A9B6C9C1959B502FCDD6C1B06EE016501B2D0BE83B89".to_owned(),
            sqlite_schema_version: 2,
        },
        notice_text: PLANFORMDB_NOTICE_TEXT.to_owned(),
        citations: vec![
            PlanformDbCitation {
                id: "lobo_2013_planform".to_owned(),
                label: "Lobo, Malone, and Levin 2013 - Planform".to_owned(),
                doi: "10.1093/bioinformatics/btt088".to_owned(),
                url: "https://doi.org/10.1093/bioinformatics/btt088".to_owned(),
            },
            PlanformDbCitation {
                id: "oviedo_2010_devbiol".to_owned(),
                label: "Oviedo et al. 2010 - long-range neural and gap-junction cues".to_owned(),
                doi: "10.1016/j.ydbio.2009.12.012".to_owned(),
                url: "https://doi.org/10.1016/j.ydbio.2009.12.012".to_owned(),
            },
        ],
        selection_policy: PlanformDbSelectionPolicy {
            selected_on: "2026-06-13".to_owned(),
            selection_basis: "Small curated PlanformDB records from source-reviewed Oviedo 2010 clusters that exercise octanol crop-position labels, VNC-disruption timing labels, and innexin RNAi crop-position labels.".to_owned(),
            non_scope: vec![
                "calibrated physiology".to_owned(),
                "Matter runtime dynamics".to_owned(),
                "PlanformDB row dump".to_owned(),
                "paper figure redistribution".to_owned(),
                "morphology image redistribution".to_owned(),
            ],
        },
        records: vec![
            planformdb_record(
                415,
                467,
                2,
                "head_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                132,
                14.0,
                &[(775, 1, "wild_type_like", 0.95), (776, 2, "double_head_two_pharynxes", 0.05)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                416,
                468,
                3,
                "pre_pharyngeal_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                118,
                14.0,
                &[(777, 1, "wild_type_like", 0.72), (778, 2, "double_head_two_pharynxes", 0.28)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                417,
                469,
                4,
                "pharyngeal_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                115,
                14.0,
                &[(779, 1, "wild_type_like", 0.50), (780, 2, "double_head_two_pharynxes", 0.50)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                418,
                470,
                20,
                "post_pharyngeal_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                145,
                14.0,
                &[(781, 2, "double_head_two_pharynxes", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                419,
                471,
                33,
                "tail_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                178,
                14.0,
                &[(782, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                441,
                493,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t0d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(824, 2, "double_head_two_pharynxes", 0.18), (825, 1, "wild_type_like", 0.82)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                442,
                494,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t0_125d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(826, 1, "wild_type_like", 0.25), (827, 2, "double_head_two_pharynxes", 0.75)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                443,
                495,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t0_5d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(828, 1, "wild_type_like", 0.80), (829, 2, "double_head_two_pharynxes", 0.20)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                444,
                496,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t1d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(830, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                446,
                498,
                2,
                "head_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                20,
                35.0,
                &[(832, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                447,
                499,
                3,
                "pre_pharyngeal_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                20,
                35.0,
                &[(833, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                448,
                500,
                4,
                "pharyngeal_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                21,
                35.0,
                &[(834, 2, "double_head_two_pharynxes", 0.20), (835, 1, "wild_type_like", 0.80)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                449,
                501,
                20,
                "post_pharyngeal_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                18,
                35.0,
                &[(836, 1, "wild_type_like", 0.20), (837, 2, "double_head_two_pharynxes", 0.80)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                450,
                502,
                33,
                "tail_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                20,
                35.0,
                &[(838, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
        ],
    };
    fixture.validate()?;
    Ok(fixture)
}

fn validate_planformdb_notice(notice_text: &str) -> Result<(), MatterFieldError> {
    for required_phrase in [
        "origin of the software and database must not be misrepresented",
        "acknowledgment and citation",
        "notice may not be removed or altered",
    ] {
        if !notice_text.contains(required_phrase) {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB notice text is missing a required phrase",
            ));
        }
    }
    Ok(())
}

fn source_dynamics_target(
    target_id: &str,
    source_ids: &[&str],
    source_target_anchor: &str,
    source_target_status: &str,
    dynamics_role: &str,
    matter_scenario_ids: &[&str],
    planformdb_record_ids: &[&str],
    allowed_uses: &[&str],
    blocked_uses: &[&str],
    checkpoints: Vec<PlanarianSourceDynamicsCheckpoint>,
) -> PlanarianSourceDynamicsTarget {
    PlanarianSourceDynamicsTarget {
        target_id: target_id.to_owned(),
        source_ids: source_ids
            .iter()
            .map(|source_id| (*source_id).to_owned())
            .collect(),
        source_target_anchor: source_target_anchor.to_owned(),
        source_target_status: source_target_status.to_owned(),
        dynamics_role: dynamics_role.to_owned(),
        matter_scenario_ids: matter_scenario_ids
            .iter()
            .map(|scenario_id| (*scenario_id).to_owned())
            .collect(),
        planformdb_record_ids: planformdb_record_ids
            .iter()
            .map(|record_id| (*record_id).to_owned())
            .collect(),
        allowed_uses: allowed_uses
            .iter()
            .map(|allowed_use| (*allowed_use).to_owned())
            .collect(),
        blocked_uses: blocked_uses
            .iter()
            .map(|blocked_use| (*blocked_use).to_owned())
            .collect(),
        checkpoints,
    }
}

fn dynamics_checkpoint(
    checkpoint_id: &str,
    source_relation: &str,
    timing_anchor: &str,
    qualitative_observation: &str,
) -> PlanarianSourceDynamicsCheckpoint {
    PlanarianSourceDynamicsCheckpoint {
        checkpoint_id: checkpoint_id.to_owned(),
        source_relation: source_relation.to_owned(),
        timing_anchor: timing_anchor.to_owned(),
        qualitative_observation: qualitative_observation.to_owned(),
        matter_boundary:
            "source-reviewed metadata only; not calibrated physiology or runtime dynamics"
                .to_owned(),
    }
}

fn head_label(
    label_id: &str,
    display_label: &str,
    label_kind: &str,
    source_relation: &str,
) -> PlanarianSpeciesLikeHeadLabel {
    PlanarianSpeciesLikeHeadLabel {
        label_id: label_id.to_owned(),
        display_label: display_label.to_owned(),
        label_kind: label_kind.to_owned(),
        source_relation: source_relation.to_owned(),
        visual_policy: "generated symbolic silhouette or text label only".to_owned(),
    }
}

fn planformdb_record(
    experiment_id: u32,
    result_set_id: u32,
    manipulation_id: u32,
    manipulation: &str,
    perturbations: &[&str],
    teaching_target: &str,
    sample_count: u32,
    regeneration_period_days: f32,
    morphologies: &[(u32, u32, &str, f32)],
    transform_notes: &[&str],
) -> PlanformDbDerivedRecord {
    PlanformDbDerivedRecord {
        record_id: format!("planformdb:experiment:{experiment_id}:resultset:{result_set_id}"),
        evidence_type: PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE.to_owned(),
        source_citation_ids: vec![
            "planformdb_250".to_owned(),
            "lobo_2013_planform".to_owned(),
            "oviedo_2010_devbiol".to_owned(),
        ],
        source_ids: PlanformDbSourceIds {
            publication_id: 1,
            species_id: 1,
            experiment_id,
            manipulation_id,
            result_set_id,
        },
        normalized_labels: PlanformDbNormalizedLabels {
            publication_source_id: "oviedo_2010_devbiol".to_owned(),
            species: "dugesia_japonica".to_owned(),
            manipulation: manipulation.to_owned(),
            perturbations: perturbations
                .iter()
                .map(|label| (*label).to_owned())
                .collect(),
            teaching_target: teaching_target.to_owned(),
        },
        assay_context: PlanformDbAssayContext {
            sample_count,
            regeneration_period_days,
        },
        resultant_morphologies: morphologies
            .iter()
            .map(
                |(resultant_morphology_id, morphology_id, normalized_outcome, frequency)| {
                    PlanformDbResultantMorphology {
                        resultant_morphology_id: *resultant_morphology_id,
                        morphology_id: *morphology_id,
                        normalized_outcome: (*normalized_outcome).to_owned(),
                        frequency: *frequency,
                    }
                },
            )
            .collect(),
        transform_notes: transform_notes
            .iter()
            .map(|note| (*note).to_owned())
            .collect(),
    }
}
