use crate::{
    MatterFieldError, PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID,
    PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID,
};

const PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE: &str = "derived_planformdb_record";
const SPECIES_LIKE_HEAD_TAXONOMY_EVIDENCE_TYPE: &str = "rights_safe_teaching_taxonomy";
const SPECIES_LIKE_HEAD_SOURCE_TARGET_ANCHOR: &str =
    "source:emmons_bell_2015_ijms::target:species_like_head_labels::future_outcome_taxonomy";
const PLANFORMDB_NOTICE_TEXT: &str = "Planform / PlanformDB Notice\n\nSource: Lobo Lab PlanformDB 2.5.0\nSource page: https://lobolab.umbc.edu/planform/download/\n\nThis Rusty Matter fixture is a tiny transformed subset of PlanformDB metadata. It does not redistribute the raw SQLite database, paper figures, or morphology images.\n\nPlanform and PlanformDB are provided as-is, without any express or implied warranty. The authors are not liable for damages arising from use of this software or database.\n\nPermission is granted to use and redistribute Planform and PlanformDB freely, subject to these restrictions:\n\n1. The origin of the software and database must not be misrepresented.\n2. Works using the software or database require acknowledgment and citation of the Planform publications.\n3. This notice may not be removed or altered from any distribution.\n\nCitation for the database/application:\n\nLobo D, Malone TJ, Levin M. Planform: an application and database of graph-encoded planarian regenerative experiments. Bioinformatics 29(8), 1098-1100, 2013. DOI: 10.1093/bioinformatics/btt088";

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
    /// Short explanation for the selected tiny subset.
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

/// Curated Matter fixture containing tiny PlanformDB-derived review records.
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

/// Builds the default tiny PlanformDB-derived fixture.
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
        scope: "Tiny reviewed PlanformDB-derived metadata fixture for Matter validation; not runtime dynamics.".to_owned(),
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
            selected_on: "2026-06-12".to_owned(),
            selection_basis: "Small PlanformDB records from one Oviedo 2010 cluster that exercise octanol, VNC disruption, and innexin RNAi labels.".to_owned(),
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
                ],
            ),
            planformdb_record(
                441,
                493,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(824, 2, "double_head_two_pharynxes", 0.18), (825, 1, "wild_type_like", 0.82)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
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
