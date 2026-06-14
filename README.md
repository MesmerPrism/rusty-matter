# Rusty Matter

Rusty Matter is the computational-matter layer for the Rusty stack. It owns
geometry, fields, SDF/TSDF grids, particle state, sampling, dynamics, schemas,
fixtures, diagnostics, and deterministic CPU reference behavior.

This repository starts with a narrow mesh, SDF, ADF, and particle foundation:

- `rusty-matter-batch`: deterministic dependency-light batch execution helpers
  for Matter CPU reference kernels, with a default serial backend, optional
  Rayon backend, stable logical chunks, and chunk-index-ordered diagnostics
  reduction;
- `rusty-matter-model`: shared model primitives and schema IDs;
- `rusty-matter-fields`: surface-field substrates, scalar/vector field
  contracts, perturbation descriptors, runtime config contracts, sparse
  fixed-step dynamics, run summaries, diagnostics, and policy-free debug
  frames/sequences over mesh sample nodes, plus qualitative bioelectric
  circuit contracts for voltage, conductance, current terms, gated coupling,
  hysteresis memory, voltage-driven readouts, realtime edit requests/results,
  circuit debug sequences, and planarian AP bioelectric presets over a
  reviewed GLB-derived educational body mesh with synthetic fallback and
  mesh-anchored substrate nodes, plus compact scenario outcome traces and
  comparison trace sets, normalized non-calibrated morphology/readout metrics,
  a rights-safe species-like head-label taxonomy, source-reviewed qualitative
  dynamics target metadata, and a small PlanformDB-derived review fixture that
  stays out of runtime dynamics;
- `rusty-matter-fields-wasm`: optional browser WebAssembly adapter over the
  Matter-owned realtime surface-field runtime and planarian bioelectric
  runtime/edit surface, including deterministic scenario resets for the
  reviewed GLB-derived planarian body substrate, GLB surface-anchor readouts,
  selected node/edge readout accessors, tiered voltage-neighborhood previews
  and mutations, bounded recent edit-event readouts, and affected-target
  readouts;
- `rusty-matter-mesh`: dynamic mesh surfaces, stable topology keys, surface
  sampling, accelerated surface-distance sampling, coordinate maps, hand
  validation mesh payloads, and dynamic mesh collider CPU reference behavior;
- `rusty-matter-sdf`: packed SDF grids and mesh-to-SDF CPU reference behavior;
- `rusty-matter-adf`: adaptive distance fields built from Matter SDF grids,
  with CPU reference construction, compact diagnostics, and deterministic
  fixtures for future acceleration parity;
- `rusty-matter-particles`: particle state, SDF interaction, spatial hashes,
  influence points, impulses, simple bodies, render-neutral payload data,
  diagnostics, fixed-step CPU simulation contracts, and batch-backed execution
  diagnostics for the general particle simulator;
- `rusty-matter-surface-runtime`: native animated-surface runtime facade over
  mesh distance, dynamic collider, batch-backed contact probes, SDF-grid
  building, and surface particles for app adapters that need the same
  Matter-owned behavior as browser previews without using Wasm;
- `rusty-matter-fixtures`: deterministic fixture validation;
- `rusty-matter-handmesh-wasm`: optional browser WebAssembly adapter over the
  Matter mesh distance sampler;
- `rusty-matter-schema`: deterministic schema catalog export.

The root files are intentionally thin. Model code is split by IDs, vectors,
bounds, mesh payloads, and errors. Field code is split by schema IDs,
substrates, scalar/vector states, perturbations, runtime configs, sparse
dynamics, bioelectric circuit contracts, circuit edit requests/results,
circuit debug frames, GLB-derived and synthetic planarian presets,
Planarian 3D realtime scenario switching, scenario outcome traces and trace
sets, normalized morphology metrics, source-evidence fixtures, selected
readouts, tiered voltage-neighborhood edit targets, recent edit-event and
affected-target readouts, summaries, debug frames, and errors.
The reviewed Planaria surface is generated as a compact Matter mesh payload
under `rusty-matter-fields/src/planarian_mesh_asset/`, with
`planarian_mesh_asset.rs` kept as the provenance and loader module.
Mesh code is split by surface, sampling/live updates, accelerated distance
queries, coordinate maps, hand payloads, dynamic collider, and error/math
helpers. Batch code is split by config, chunks, executor, report, and errors.
SDF code is split by builder, config, packed grid, geometry helpers, and
errors. ADF code is split by builder, config, field, and errors. Particle code
is split by IDs, state, render-neutral payloads, configs,
interactions, spatial hash, diagnostics, simulation, and errors. Surface
runtime code is split by facade/runtime types and orchestration errors.
Fixture generation is split by artifact dispatch,
summaries, fields, SDF, ADF, mesh, particle, and damaged-input families.
Schema export is split by catalog, CLI, and error modules.

## Bioelectricity Source Targets

The planarian bioelectric scenarios carry public source/target anchor IDs in
their serialized `literature_anchors` field. See
`docs/BIOELECTRICITY_SOURCE_TARGETS.md` before changing planarian scenario
claims, adding source-derived thresholds, or promoting PlanformDB-derived
records into fixtures.
The current PlanformDB-derived fixture is a small validation/review artifact
with notice text and damaged-input coverage; it is not a runtime dynamics input
or a calibrated phenotype predictor.
The source-reviewed dynamics-target fixture maps high-confidence literature
targets to existing synthetic educational scenarios and future fixture gates;
it is metadata/validation evidence, not a new solver or calibrated physiology
claim.

## Boundary

Matter is data and simulation authority. It does not own command routing,
operator UI, renderer backend imports, platform hand mesh acquisition, Quest
tooling, or downstream app behavior.

Default schema IDs use `rusty.matter.*`.

## Validation

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```
