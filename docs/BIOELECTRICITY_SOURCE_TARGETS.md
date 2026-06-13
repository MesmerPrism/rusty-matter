# Bioelectricity Source Targets

This note records the public-safe source IDs used by the planarian
bioelectricity slice. It is a source-target map for implementation work, not a
claim that the current fixtures reproduce experimental data.

Rusty Matter currently owns a qualitative educational substrate: mesh-surface
samples, normalized voltage, conductance edges, memory/readout layers,
scenario runs, and outcome traces. The source IDs below identify the papers or
datasets that motivate each target. They do not authorize calibrated constants,
paper-figure reuse, or raw dataset import.

## Current Serialized Anchors

Planarian scenario fixtures include compact `literature_anchors` strings with
this shape:

```text
source:<source_id>::target:<target_id>[::future_*]
```

Current source IDs:

| Source ID | Public source | Implementation role |
| --- | --- | --- |
| `durant_2019_bpj` | <https://doi.org/10.1016/j.bpj.2019.01.029> | Early transient depolarization, washout, and memory/control scenarios. |
| `beane_2011_chembiol` | <https://doi.org/10.1016/j.chembiol.2010.11.012> | Head-vs-tail membrane-voltage teaching context and future voltage/pump annotations. |
| `oviedo_2010_devbiol` | <https://doi.org/10.1016/j.ydbio.2009.12.012> | Gap-junction-mediated polarity and coupling/blockade scenario context. |
| `beane_2013_dev` | <https://doi.org/10.1242/dev.086900> | Future head/organ-size readout metrics; current code must not claim calibrated morphology. |
| `emmons_bell_2015_ijms` | <https://doi.org/10.3390/ijms161126065> | Categorical species-like head outcome labels and future derived mappings; current code uses only rights-safe labels and qualitative gap-block context. |
| `planformdb_250` | <https://lobolab.umbc.edu/planform/download/> | Curated experiment/phenotype review fixtures after schema, citation, and notice review. |

The private source archive and page-level extraction notes live outside this
public repository. Use the Bioelectricity hub's tracked source index and private
source archive when stronger targets are needed.

The optional browser Wasm adapter exposes the active scenario's
`evidence_type`, `expected_outcome`, `literature_anchors`, and voltage-unit
policy as read-only metadata for Optics. That export surface is an annotation
contract over Matter-owned state; it does not add calibrated dynamics or
renderer-owned simulation semantics.

Source-reviewed dynamics targets are serialized in
`fixtures/fields/planarian-source-dynamics-targets.json`. That fixture maps
high-confidence literature targets to current synthetic scenarios, future
fixture gates, and explicit blocked uses; it does not change the Matter
runtime or calibrate the existing synthetic dynamics.

Head-size and species-like head-label targets are scoped in
`docs/PLANARIAN_METRIC_TARGETS.md`. The current Matter slice implements the
first non-calibrated metric/taxonomy fixtures only; stronger morphometric
thresholds, generated silhouette assets, source-fit values, and stochastic
outcome behavior remain gated.

## Target Status

| Target ID | Current status in Matter | Gate before strengthening |
| --- | --- | --- |
| `ap_transient_memory` | Implemented as qualitative `TransientDepolarizationMemory` and `TransientDepolarizationNoMemoryControl` scenarios. | Extract source-derived timing/value targets before numeric thresholds or prediction claims. |
| `gap_block_conductance` | Implemented as qualitative `GapBlock` conductance/coupling reduction and outcome trace comparison. | Extract figure/table targets before claiming source-fit behavior. |
| `head_vs_tail_voltage` | Represented as normalized AP voltage/readout context with read-only Wasm annotation/unit-policy metadata for Optics. | Source-review exact assays/values before named pump/channel constants or calibrated millivolt fixtures. |
| `persistent_axis_recut_history` | Registered as a future source-reviewed dynamics target only. | Add session/package history fixtures before claiming persistence across repeated regeneration rounds. |
| `head_size_scaling` | Implemented as `PlanarianNormalizedMorphologyMetrics` with mesh-normalized AP-region/readout extents and no thresholds. | Extract area/location targets before claiming source-fit head or organ size. |
| `species_like_head_labels` | Implemented as a rights-safe categorical teaching taxonomy fixture with generated-symbol/text label policy. | Add generated silhouettes or source-derived category frequencies only after a rights/provenance review. |
| `planformdb_curated_subset` | Implemented as a tiny Matter review fixture with source IDs, citations, notice text, normalized labels, and damaged-input tests. | Keep it out of runtime dynamics; expand only through curated, traceable derived records. |

## Implementation Rules

- Keep normalized educational scenarios separate from source-fitted calibration.
- Add public source IDs and DOI links to fixtures and docs; keep local PDF,
  page-render, crop, and database paths out of this repo.
- Prefer new derived fixture files over mutating runtime dynamics when the next
  step is source review.
- Treat PlanformDB as experiment/phenotype evidence, not as Matter runtime
  authority. The current derived fixture validates metadata and frequencies
  only; it does not drive circuit dynamics or stochastic predictions.
- Keep Optics as a consumer of Matter-owned source/target anchors. Optics may
  display source labels and claim boundaries, but it must not invent simulation
  semantics.

## Validation

After changing source anchors, planarian scenario kinds, or target status,
regenerate and validate fixtures:

```powershell
cargo run -p rusty-matter-fixtures -- write
cargo run -p rusty-matter-fixtures -- validate
cargo test -p rusty-matter-fields
```
