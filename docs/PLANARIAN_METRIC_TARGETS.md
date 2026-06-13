# Planarian Metric Targets

This note defines the source-derived metric and outcome-taxonomy targets for
the qualitative Planarian AP bioelectric slice. It records the first
implemented Matter fixtures and the gates that still block calibrated
morphology claims.

Rusty Matter currently owns normalized voltage, conductance, memory/readout
state, deterministic scenario runs, and compact outcome traces. The current
metric trace columns are educational checks over those Matter-owned states:
posterior memory, posterior head identity, head-region head identity,
tail-region tail identity, cut-band voltage, and cross-cut conductance.

## Head-Size Scaling

Source target:

```text
source:beane_2013_dev::target:head_size_scaling::future_metric
```

Implementation posture:

- Matter now emits `PlanarianNormalizedMorphologyMetrics` as a first
  normalized, explicitly educational `planarian_metrics` fixture;
- do not add head-size thresholds, organ-size thresholds, or millivolt
  calibration from the current qualitative circuit state;
- keep the metric in Matter if it summarizes geometry, readout, region extent,
  or deterministic scenario output;
- let Optics display the metric only after Matter emits the value and source
  target anchor.

Implemented first fields:

- `head_region_extent_normalized`;
- `head_identity_extent_normalized`;
- `pharyngeal_region_extent_normalized`;
- `source_target_status`.

Blocked gates before strengthening:

- source figure/table extraction with rights-safe derived values;
- explicit region definitions for any anatomical extent;
- area/location metrics beyond the current AP-region/readout extents;
- source-derived thresholds or pass/fail targets.

## Species-Like Head Labels

Source target:

```text
source:emmons_bell_2015_ijms::target:species_like_head_labels::future_outcome_taxonomy
```

Implementation posture:

- Matter now emits this as a categorical teaching taxonomy fixture, not as a
  stochastic predictor;
- prefer generated silhouettes or simple symbolic labels;
- do not copy paper figure crops or thumbnails into Matter, Optics, fixtures,
  or public docs without a separate rights review;
- keep labels separate from quantitative morphology metrics until source-
  derived morphometric tables exist;
- let Optics show categories as a legend only after Matter or a derived fixture
  owns the category identifiers.

Candidate future labels:

- `normal_head`;
- `posterior_head_identity_shift`;
- `double_head_like`;
- `species_like_head_shape_category`;
- `unclassified_teaching_abstraction`.

Implemented first fixture:

- `fixtures/fields/planarian-species-like-head-taxonomy.json`;
- `PlanarianSpeciesLikeHeadTaxonomy` validates source target, no-paper-image
  policy, label uniqueness, and the required unclassified teaching category.

Blocked gates before strengthening:

- generated silhouette/icon plan;
- source-derived or curated category mapping;
- explicit synthetic-vs-derived fixture labels.

## PlanformDB-Derived Review Fixture

Source target:

```text
source:planformdb_250::target:planformdb_curated_subset
```

Matter now carries `fixtures/fields/planformdb-derived-v0.json`, a tiny
PlanformDB-derived review fixture with source IDs, citations, notice text,
normalized manipulation/outcome labels, sample counts, regeneration periods,
and resultant morphology frequencies. It is a metadata and validation fixture
only. It does not import raw rows, source images, conductance constants,
runtime dynamics, or stochastic predictors.

Validation includes damaged cases for invalid frequency normalization and
fixture metadata in the normal `rusty-matter-fixtures` route.

## Non-Scope

Do not use this note to add:

- calibrated electrophysiology;
- source-paper images;
- PlanformDB raw records;
- morphometric thresholds without extracted source targets;
- browser-owned metric computation.

The DiffeoMorph shape-loss/controller lane is deferred from this Matter metric
slice. It remains a separate morphology/control reference path until a future
task explicitly reopens it.

The next executable step should be either a derived source-target extraction
artifact in the Bioelectricity hub or a Matter-owned normalized metric fixture
with damaged-input tests.
