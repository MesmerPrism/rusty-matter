# Rusty Matter Mesh Lab

Rusty Matter Mesh Lab is the Matter-owned workflow family for mesh intake,
normalization, deterministic mesh editing, surface sampling, coordinate-map
generation, validation, and package export.

The goal is to keep reusable mesh behavior in one place so Kuramoto mesh
experiments, Planarian bioelectric substrates, Meta hand mesh validation, SDF
builders, particle surface paths, and future Morphospace projects do not
rebuild the same mesh-coordinate machinery in app-specific code.

## Decision

Mesh Lab is a set of Matter contracts and tools around the existing
`rusty-matter-mesh` crate. It is not a renderer, app runtime, command broker,
or platform acquisition layer.

Rusty Matter owns:

- canonical triangle surfaces;
- source/provenance descriptors for imported or generated meshes;
- deterministic mesh edit recipes and edit reports;
- surface samples, barycentric anchors, coordinate maps, and local frames;
- optional same-surface and cross-surface neighborhood artifacts;
- validation summaries and damaged-input expectations;
- fixture and package artifacts that downstream crates can consume.

Rusty Matter does not own:

- Kuramoto oscillator state;
- Planarian scenario policy or biological claims beyond Matter-owned
  qualitative field/circuit contracts;
- Meta/Quest hand mesh acquisition or XR runtime lifecycle;
- cameras, materials, colors, renderer buffers, shaders, or visual scorecards;
- command/session/stream authority.

## Existing Foundation

The current repository already has the core mesh pieces:

- `crates/rusty-matter-mesh/src/surface.rs`: `TriangleMeshSurface` and topology
  keys;
- `crates/rusty-matter-mesh/src/sampling.rs`: deterministic surface samples,
  barycentric anchors, live topology updates, same-surface neighbor tiers, and
  cross-surface neighborhoods;
- `crates/rusty-matter-mesh/src/coordinate.rs`: `MeshCoordinateMap` and local
  coordinate frames;
- `crates/rusty-matter-mesh/src/hand.rs`: hand validation payloads over the
  same generic triangle surface contract;
- `tools/convert_planarian_glb_surface.py`,
  `tools/extract_glb_mesh_surfaces.py`, and
  `tools/extract_glb_mesh_surface_sequence.py`: existing external mesh intake
  scripts that should be consolidated behind Mesh Lab contracts over time.

## Target Module Shape

The long-term Matter shape should stay split by durable authority:

```text
crates/rusty-matter-mesh/
  src/surface.rs        # canonical triangle surface and topology identity
  src/source.rs         # source descriptors, provenance, units, format hints
  src/edit.rs           # deterministic mesh edits and repair reports
  src/sampling.rs       # sample sets, barycentric anchors, neighborhoods
  src/coordinate.rs     # coordinate maps and local frames
  src/package.rs        # coordinate-map packages for downstream consumers
  src/hand.rs           # hand-specific wrappers over generic mesh contracts
  src/distance.rs       # accelerated closest-surface queries
  src/collider.rs       # dynamic collider reference behavior
```

Future command-line tooling should live outside the core crate:

```text
crates/rusty-matter-mesh-lab/
  src/main.rs           # dispatch only
  src/cli.rs
  src/inspect.rs
  src/import.rs
  src/edit.rs
  src/sample.rs
  src/package.rs
  src/validate.rs
  src/preview.rs
```

The CLI should call the same library contracts used by fixtures and tests.
UI tools can wrap the CLI/API later; they should not become a separate mesh
authority.

## Core Contracts

`MeshSourceDescriptor` records where a mesh came from and how it should be
interpreted:

```text
schema_id
source_id
source_uri
source_format
source_hash
license
attribution
unit_scale_to_meters
axis_convention
notes
```

`MeshCoordinateMapPackage` binds a source descriptor, canonical
`TriangleMeshSurface`, and `MeshCoordinateMap` into one inspectable artifact:

```text
schema_id
package_id
source
surface
coordinate_map
notes
```

The coordinate map remains mesh-attached:

```text
position
normal
triangle_index
barycentric
optional tangent/bitangent through local frames
optional same-surface neighbors
optional cross-surface neighbors
optional labels, masks, or regions
```

Neighbors are optional companion data. Kuramoto and Planarian workflows may
need neighbor tiers. Static number glyphs can explicitly request no neighbors.
Same-surface tiers use a Matter-owned approximate surface-walk metric: sampled
barycentric points become graph nodes, triangle faces get complete in-face
links across vertices and refined edge-subdivision connectors, then Dijkstra
ranks candidate samples by travel along that mesh-walk graph instead of by
straight world-space chord distance.

## Mesh Editing Scope

Mesh Lab editing should be deterministic and evidence-producing. Initial edit
steps should be small and auditable:

- transform, scale, and axis remap;
- merge duplicate vertices with a tolerance;
- remove degenerate triangles;
- remove unreferenced vertices;
- split connected components;
- reverse winding or recompute/report normals;
- select a component or material/primitive from a multi-mesh source;
- normalize origin, bounds, or unit scale.

Every edit recipe should produce an edit report with source hash, output hash,
counts before/after, warnings, and rejected operations. Destructive or
heuristic repairs should remain explicit recipe steps, not hidden importer
behavior.

## Import Adapter Strategy

Mesh Lab should accept new formats through adapters that normalize into
`TriangleMeshSurface`.

Initial adapters:

- procedural/test surfaces;
- STL for static assets such as number glyphs;
- existing GLB extraction paths for Planarian and Meta hand validation.

Later adapters:

- OBJ;
- glTF scene/material selection;
- animated GLB surface sequences;
- planar grids and image-derived surfaces;
- external simulation or segmentation meshes.

Import adapters are tooling. Runtime crates consume canonical Matter surfaces
and coordinate-map packages, not arbitrary source files.

## Project Consumers

Kuramoto mesh work consumes coordinate maps plus same-surface surface-walk
neighborhoods for oscillator coupling. It should not own mesh sampling.

Planarian bioelectricity consumes mesh-anchored substrate nodes and optional
neighbor tiers for surface fields and qualitative circuit work. It should not
own GLB conversion logic beyond selecting reviewed source assets.

Meta hand mesh work consumes bind/rest surfaces, validation frames, and
animated surface sequences. Shared sampling, distance, collider, SDF, and
coordinate-map behavior stays generic in Matter; Lattice owns tracked reference
spaces and poses.

Optics can render previews and validation overlays, but it references Matter
payloads instead of deriving its own mesh truth.

## Validation Slots

Mesh Lab validation should cover:

- source descriptor schema, non-empty IDs, format, license, attribution, and
  positive unit scale;
- finite positions and valid triangle indices;
- positive surface area;
- stable topology key expectations;
- sample count and deterministic sample generation;
- barycentric reconstruction and barycentric sum;
- unit normals and local frame orthogonality;
- optional neighbor target validity;
- package/source/surface/coordinate-map consistency;
- damaged source descriptors, damaged packages, and damaged edit recipes.

Repo-local validation remains:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

Targeted iteration commands:

```powershell
cargo test -p rusty-matter-mesh
cargo run -p rusty-matter-fixtures -- validate
cargo run -p rusty-matter-schema -- export --check
```

## Initial Implementation Plan

1. Add `MeshSourceDescriptor` and `MeshCoordinateMapPackage` contracts around
   existing `TriangleMeshSurface` and `MeshCoordinateMap`.
2. Add a no-neighbor sample-config helper for static coordinate maps.
3. Add tests that package a static coordinate map and reject inconsistent
   package/source inputs.
4. Add a fixture summary proving the package contract is generated and
   validated with the existing fixture route.
5. Update schema catalog, architecture docs, fixture docs, and this document.
6. Add STL static-number intake as the first real external Mesh Lab workflow.
7. Consolidate existing Planarian and hand GLB scripts behind the same source
   descriptor and package terminology.
8. Add deterministic edit recipes and reports.
9. Add a dedicated `rusty-matter-mesh-lab` CLI crate once the library contracts
   have two or more real workflows.

## Implementation Notes

### 2026-06-15 Iteration 1

Status: complete for the first contract slice.

Intent:

- create this dedicated Mesh Lab planning and implementation document;
- implement the first contract slice in `rusty-matter-mesh`;
- keep the first slice schema/data-only and CPU-only;
- avoid building the full importer/editor CLI before the shared contracts are
  protected by tests and fixtures.

Deferred:

- STL intake for the 0-9 number meshes;
- blue-noise or Poisson-disk thinning;
- deterministic edit recipes;
- `rusty-matter-mesh-lab` CLI crate;
- Optics preview surfaces.

Implemented so far:

- added `MeshSourceDescriptor` in `crates/rusty-matter-mesh/src/source.rs`;
- added `MeshCoordinateMapPackage` in
  `crates/rusty-matter-mesh/src/package.rs`;
- added `MeshSurfaceSampleConfig::without_neighbors()` for static coordinate
  maps;
- added unit tests for no-neighbor package validation, damaged source
  metadata, and mismatched package surfaces;
- added `fixtures/mesh/unit-square-coordinate-map-package-summary.json`;
- added schema catalog entries for the source descriptor, coordinate-map
  package, and package fixture summary.

Validation so far:

- `cargo test -p rusty-matter-mesh` passed with 18 tests.
- `cargo fmt --all --check` passed.
- `cargo run -p rusty-matter-fixtures -- validate` passed with 47 artifacts.
- `cargo run -p rusty-matter-schema -- export --check` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1`
  passed.

Next implementation slice:

- add the first external static mesh intake workflow for the permissive 0-9
  STL number set, producing one no-neighbor coordinate-map package per digit;
- keep source attribution and license metadata in `MeshSourceDescriptor`;
- validate 10,000 coordinates per digit, finite normals, barycentric anchors,
  and no same-surface neighbor output unless explicitly requested.
