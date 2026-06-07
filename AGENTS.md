# Rusty Matter Agent Notes

This is the clean source repository for Rusty Matter. Keep committed content
self-contained and free of private planning paths, downstream app names,
platform-specific runtime handles, and historical naming drift.

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
- Renderer adapters own GPU buffers, shaders, draw calls, texture atlases,
  platform frame lifecycle, and backend imports.
- XR and device adapters own platform hand mesh acquisition and device runtime
  lifecycle. They convert platform frames into Matter payloads.
- Keep Kuramoto, oscillator coupling, breath control, study defaults, and
  downstream visual-driver behavior out of Matter unless a later explicit
  generalization decision is recorded.
- Use `rusty.matter.*` schema IDs for default Matter contracts. Legacy XR names
  may appear only in explicitly named compatibility layers outside Matter core.
- Add fixtures and damaged-input expectations before runtime adapters.
- Keep high-rate particle arrays and grids out of command/control JSON routes;
  use artifacts, bounded summaries, or data-plane adapters.

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
- `rusty-matter-mesh/src/lib.rs`: facade over `surface`, `sampling`,
  `coordinate`, `hand`, `collider`, accelerated surface `distance`, `error`,
  and shared `math`.
- `rusty-matter-sdf/src/lib.rs`: facade over `builder`, `config`,
  `grid`, `geometry`, and `error`.
- `rusty-matter-particles/src/lib.rs`: facade over `ids`, `state`, `render`,
  `config`, `interactions`, `spatial_hash`, `diagnostics`, `simulator`, and
  `error`.
- `rusty-matter-handmesh-wasm/src/lib.rs`: target-gated browser adapter over
  the shared Matter hand-mesh distance sampler.
- `rusty-matter-fixtures/src/main.rs`: dispatch-only binary over `cli`,
  `artifact`, `summary`, `sdf`, `mesh`, `particles`, `damaged`, and `error`.
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
