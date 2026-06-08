# Rusty Matter

Rusty Matter is the computational-matter layer for the Rusty stack. It owns
geometry, fields, SDF/TSDF grids, particle state, sampling, dynamics, schemas,
fixtures, diagnostics, and deterministic CPU reference behavior.

This repository starts with a narrow mesh, SDF, and particle foundation:

- `rusty-matter-model`: shared model primitives and schema IDs;
- `rusty-matter-fields`: surface-field substrates, scalar/vector field
  contracts, perturbation descriptors, runtime config contracts, sparse
  fixed-step dynamics, run summaries, diagnostics, and policy-free debug
  frames/sequences over mesh sample nodes, plus qualitative bioelectric
  circuit contracts for voltage, conductance, current terms, gated coupling,
  hysteresis memory, voltage-driven readouts, realtime edit requests/results,
  circuit debug sequences, and synthetic planarian AP bioelectric presets;
- `rusty-matter-fields-wasm`: optional browser WebAssembly adapter over the
  Matter-owned realtime surface-field runtime and planarian bioelectric
  runtime/edit surface;
- `rusty-matter-mesh`: dynamic mesh surfaces, stable topology keys, surface
  sampling, accelerated surface-distance sampling, coordinate maps, hand
  validation mesh payloads, and dynamic mesh collider CPU reference behavior;
- `rusty-matter-sdf`: packed SDF grids and mesh-to-SDF CPU reference behavior;
- `rusty-matter-particles`: particle state, SDF interaction, spatial hashes,
  influence points, impulses, simple bodies, render-neutral payload data,
  diagnostics, and fixed-step CPU simulation contracts;
- `rusty-matter-fixtures`: deterministic fixture validation;
- `rusty-matter-handmesh-wasm`: optional browser WebAssembly adapter over the
  Matter mesh distance sampler;
- `rusty-matter-schema`: deterministic schema catalog export.

The root files are intentionally thin. Model code is split by IDs, vectors,
bounds, mesh payloads, and errors. Field code is split by schema IDs,
substrates, scalar/vector states, perturbations, runtime configs, sparse
dynamics, bioelectric circuit contracts, circuit edit requests/results,
circuit debug frames, synthetic planarian presets, summaries, debug frames, and
errors.
Mesh code is split by surface,
sampling/live updates, accelerated
distance queries, coordinate maps, hand payloads, dynamic collider, and
error/math helpers. SDF code is split by builder, config, packed grid, geometry
helpers, and errors. Particle code is split by IDs, state, render-neutral
payloads, configs, interactions, spatial hash, diagnostics, simulation, and
errors. Fixture generation is split by artifact dispatch, summaries, fields,
SDF, mesh, particle, and damaged-input families. Schema export is split by
catalog, CLI, and error modules.

## Boundary

Matter is data and simulation authority. It does not own command routing,
operator UI, renderer backend imports, platform hand mesh acquisition, Quest
tooling, or downstream app behavior.

Default schema IDs use `rusty.matter.*`.

## Validation

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```
