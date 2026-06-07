# Architecture

Rusty Matter is the source of truth for computational matter payloads and
deterministic CPU reference behavior.

## Ownership

Matter owns:

- mesh and geometry payloads;
- dynamic mesh topology keys, surface samples, coordinate maps, and local
  coordinate frames;
- hand rig, joint-frame, and validation-mesh payload shapes that convert to a
  generic triangle mesh surface;
- accelerated closest-surface distance samplers over the current triangle mesh;
- fields and packed SDF grids;
- particle state and dynamics;
- render-neutral particle payload data derived from Matter state;
- deterministic CPU reference algorithms;
- fixture and schema artifacts;
- diagnostics for simulation and field operations.

Matter does not own:

- command/session/stream authority;
- renderer backend imports;
- view and appearance policy;
- platform hand mesh acquisition;
- XR runtime lifecycle;
- downstream app-specific behavior.

## Matter / Optics Boundary

Matter may prepare deterministic render-neutral payloads when the payload is a
direct, policy-free projection of Matter state. For particles, that means IDs,
positions, radii, velocities, speed, age, flags, time, and bounds.

Matter must not own view selection, cameras, projections, colors, materials,
opacity, blending, sprites, billboarding, lighting, depth policy, or renderer
quality scorecards. Those belong in Rusty Optics or renderer adapters. Optics
can reference Matter payload IDs and schema IDs, but it should not duplicate
particle simulation truth.

## Mesh Authority

`rusty-matter-mesh` is the shared mesh contract crate. Generic mesh behavior
uses `TriangleMeshSurface`, `MeshSurfaceTopologyKey`, `MeshSurfaceSampleSet`,
`MeshCoordinateMap`, and `DynamicMeshCollider`. Hand-specific recording payloads
wrap the same surface contract instead of creating a separate hand-only mesh
authority.

This means a platform hand provider, a PC playback/export tool, the SDF builder,
the dynamic mesh collider, and particle/coordinate-map consumers all agree on
the same topology hash, triangle indices, vertex positions, barycentric sample
anchors, and local frames. Hand animation recording/export can remain
hand-specific, while reusable mesh sampling, SDF, collider, and coordinate
distribution code stays generic.

## Current Slices

The implemented foundation slices are intentionally CPU/data-only:

- model primitives;
- triangle mesh validation;
- dynamic mesh surface validation and topology keys;
- deterministic surface sampling with same-surface neighbor tiers;
- mesh coordinate maps and local displacement frames;
- hand validation mesh frames over the generic mesh surface contract;
- dynamic mesh collider surface inflation, closest-point, and sphere-overlap
  reference behavior backed by the shared accelerated distance sampler;
- packed SDF grid contracts;
- mesh-to-SDF CPU reference builder;
- particle state and SDF interaction contracts;
- fixed-step particle simulation;
- render-neutral particle payload preparation;
- deterministic spatial hash neighbor queries;
- particle influence points, one-shot impulses, and simple sphere/AABB bodies;
- particle diagnostics for SDF, neighbor, influence, impulse, and body work;
- fixture and schema catalog checks;
- dependency and namespace boundary guards.

Runtime workers, renderer-owned GPU state, platform adapters, command routing,
and downstream visual-driver behavior stay outside Matter.

The optional `rusty-matter-handmesh-wasm` crate is a thin browser export over
the same Matter mesh distance sampler used natively. It does not own renderer
policy, browser UI, command routing, or hand-mesh acquisition.

## Module Map

Crate roots stay as facades so Matter does not rebuild the monolithic
`main.rs` and `lib.rs` shapes removed elsewhere in the Rusty refactor.

- `rusty-matter-mesh/src/surface.rs`: generic triangle surface and topology key.
- `rusty-matter-mesh/src/sampling.rs`: deterministic surface sampling,
  barycentric anchor updates, live sampler updates, and cross-neighborhoods.
- `rusty-matter-mesh/src/distance.rs`: accelerated closest-surface sampling
  over dynamic triangle meshes, including query diagnostics for node and exact
  triangle tests.
- `rusty-matter-mesh/src/coordinate.rs`: coordinate-map frame configs, local
  frames, and coordinate maps.
- `rusty-matter-mesh/src/hand.rs`: hand rig, joint-frame, and validation mesh
  wrappers over the shared generic surface contract.
- `rusty-matter-mesh/src/collider.rs`: dynamic mesh collider config, update,
  diagnostic shell, closest-point, and sphere-overlap reference behavior over
  the shared distance sampler.
- `rusty-matter-model/src/ids.rs`: dotted IDs and Matter schema IDs.
- `rusty-matter-model/src/vec3.rs`: vector math primitives.
- `rusty-matter-model/src/bounds.rs`: axis-aligned bounds.
- `rusty-matter-model/src/mesh.rs`: triangle mesh snapshot payloads.
- `rusty-matter-sdf/src/builder.rs`: mesh-to-SDF CPU reference builder.
- `rusty-matter-sdf/src/config.rs`: mesh-to-SDF builder configuration.
- `rusty-matter-sdf/src/grid.rs`: packed SDF grid and nearest-cell sampling.
- `rusty-matter-sdf/src/geometry.rs`: private triangle distance helpers.
- `rusty-matter-particles/src/state.rs`: particle state and set snapshots.
- `rusty-matter-particles/src/render.rs`: render-neutral particle payloads for
  Optics or renderer adapters.
- `rusty-matter-particles/src/interactions.rs`: influence points, impulses,
  simple bodies, and interaction bundles.
- `rusty-matter-particles/src/spatial_hash.rs`: deterministic neighbor
  candidate hash.
- `rusty-matter-particles/src/simulator.rs`: fixed-step CPU reference
  simulation.
- `rusty-matter-handmesh-wasm/src/web.rs`: `wasm-bindgen` adapter that accepts
  typed-array mesh buffers and exposes accelerated closest-surface samples plus
  sampler diagnostics for browser previews.
- `rusty-matter-fixtures/src/main.rs`: dispatch-only entrypoint; fixture
  families live in `sdf`, `mesh`, `particles`, `damaged`, `summary`, and
  `artifact` modules.
- `rusty-matter-schema/src/main.rs`: dispatch-only entrypoint; schema catalog,
  CLI parsing, and error display live in separate modules.

Future Matter/Manifold package bridge work should use a dedicated package or
bridge module that references Matter schema IDs and fixture artifacts. It
should not duplicate SDF, mesh, particle, or sampling algorithms in package
descriptors.
