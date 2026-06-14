# Rusty Matter Agent Notes

This is the clean source repository for Rusty Matter. Keep committed content
self-contained and free of local-only planning paths, downstream app names,
platform-specific runtime handles, and historical naming drift.

Rusty Morphospace is the top-level project/platform umbrella. This repo remains
the Matter lane inside that umbrella: morphology of computational substance,
including fields, geometry, meshes, particles, SDF/TSDF, sampling, dynamics,
fixtures, and deterministic CPU reference behavior. Do not introduce
`rusty.morphospace.*` schemas here; use `rusty.matter.*` for Matter contracts.

Project-owned source in this repo is licensed `AGPL-3.0-or-later`. Keep
third-party dependencies, datasets, captured geometry, GLB or mesh assets,
research-derived fixtures, binary releases, and external tools under their own
provenance and notice requirements; see `docs/LICENSING.md`.

## Purpose

Rusty Matter owns computational matter: fields, geometry, particles, SDF/TSDF,
sampling, dynamics, fixtures, schemas, and deterministic CPU reference behavior.

It should remain usable without UI frameworks, renderer backends, platform SDKs,
XR runtimes, headset tooling, device APIs, dynamic plugin loading, runtime
sockets, media stacks, or downstream app crates.

## Read Order

1. `README.md`
2. `docs/ARCHITECTURE.md`
3. `docs/VALIDATION.md`
4. `fixtures/README.md`

## Architecture Rules

- Matter owns data truth and deterministic CPU reference algorithms for meshes,
  fields, SDFs, particles, samples, dynamics, and diagnostics.
- Command/session/stream authority belongs outside Matter. Packages and routes
  may reference Matter schema IDs, but they do not own Matter algorithms.
- Optics owns view, projection, appearance policy, debug visualization, and
  visual scorecards. Matter may prepare deterministic render-neutral payloads
  for fixtures and interchange, but it does not own renderer policy.
- Lattice owns situated relation snapshots: reference spaces, transforms,
  tracked poses, view sets, spatial input roles, frame-state binding,
  calibration, validity, confidence, and runtime capability evidence. Matter
  artifacts may be referenced from Lattice, but Matter owns the computational
  substance those relations point at.
- Renderer adapters own GPU buffers, shaders, draw calls, texture atlases,
  platform frame lifecycle, and backend imports.
- XR/device adapters own platform hand mesh acquisition and device runtime
  lifecycle. They convert platform frames into Matter payloads and Lattice
  relation snapshots outside Matter core.
- Keep app-specific dynamics, control bindings, study defaults, and downstream
  visual-driver behavior out of Matter unless a later explicit generalization
  decision is recorded.
- Use `rusty.matter.*` schema IDs for default Matter contracts. Legacy XR names
  may appear only in explicitly named compatibility layers outside Matter core.
- Add fixtures and damaged-input expectations before runtime adapters.
- Keep high-rate particle arrays and grids out of command/control JSON routes;
  use artifacts, bounded summaries, or data-plane adapters.
- Keep particle integration/render cadence, particle force-source refresh,
  animated hand-surface updates, and SDF/ADF field builds as independent
  runtime clocks. Normal profiling selects exactly one particle-force
  authority at a time: `mesh-distance`, `sdf-field`, `adf-field`, or `none`.
  Dual mesh/field computation is a bounded
  compare-probe diagnostic only, never the default particle path.

## File Organization Rules

- Keep `src/lib.rs` files as facades: module declarations, public reexports,
  and short crate-level docs only. Do not add new mesh, particle, SDF, schema,
  or bridge behavior directly to a crate root.
- Keep binary `src/main.rs` files as dispatch-only entrypoints. CLI parsing,
  artifact generation, validation checks, and fixture-family code belong in
  modules that mirror the ownership family.
- Split before adding behavior when a file starts mixing independent families,
  even below the global 10k-line pressure threshold. For Matter, the important
  families are surface/topology, sampling/live updates, coordinate maps, hand
  payloads, dynamic colliders, particle state/render payloads, interactions,
  spatial hashing, diagnostics, simulation, schema catalog export, and fixture
  generation.
- Preserve public names, schema IDs, serde field names, fixture outputs, CLI
  messages, validation outcomes, and dependency boundaries during mechanical
  splits. Validate with `.\tools\check_all.ps1` before continuing a feature
  slice.

Current crate-root maps:

- `rusty-matter-model/src/lib.rs`: facade over `ids`, `vec3`,
  `bounds`, `mesh`, and `error`.
- `rusty-matter-fields/src/lib.rs`: facade over `ids`, `substrate`, `state`,
  `perturbation`, `config`, `runtime`, `dynamics`, `circuit`,
  `circuit_edit`, `circuit_debug`, `planarian`, `planarian_metrics`,
  `planarian_evidence`, `summary`, `debug_frame`, and `error`.
  `planarian_mesh_asset.rs` is a small generated loader over the compact
  `planarian_mesh_asset/planaria_sketchfab_surface.bin` payload; do not add
  behavior to either generated surface file.
- `rusty-matter-fields-wasm/src/lib.rs`: target-gated browser adapter over the
  Matter surface-field runtime.
- `rusty-matter-mesh/src/lib.rs`: facade over `surface`, `sampling`,
  `coordinate`, `hand`, `collider`, accelerated surface `distance`, `error`,
  and shared `math`.
- `rusty-matter-sdf/src/lib.rs`: facade over `builder`, `config`,
  `grid`, `geometry`, and `error`.
- `rusty-matter-adf/src/lib.rs`: facade over `builder`, `config`, `field`
  including runtime ADF lookup indexes, and `error`.
- `rusty-matter-particles/src/lib.rs`: facade over `ids`, `state`, `render`,
  `config`, `interactions`, `spatial_hash`, `diagnostics`, `simulator`, and
  `error`.
- `rusty-matter-handmesh-wasm/src/lib.rs`: target-gated browser adapter over
  the shared Matter hand-mesh distance sampler.
- `rusty-matter-fixtures/src/main.rs`: dispatch-only binary over `cli`,
  `artifact`, `summary`, `fields`, `sdf`, `adf`, `mesh`, `particles`,
  `damaged`, and `error`.
- `rusty-matter-schema/src/main.rs`: dispatch-only binary over `cli`,
  `catalog`, and `error`.

For Manifold package bridge work, add Matter package/artifact descriptor logic
to a focused bridge or package module. Do not put bridge logic into mesh,
particle, SDF, or fixture facades, and do not duplicate Matter math inside a
Manifold package descriptor.

## Validation

Run narrow checks before committing a slice:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

The check script runs formatting, tests, fixture validation, schema catalog
checks, and boundary scans.
