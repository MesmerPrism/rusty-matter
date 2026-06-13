# Validation

Run the narrow repo-local check before committing:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

The check covers:

- `cargo fmt --all --check`;
- `cargo test --workspace`;
- `cargo test -p rusty-matter-batch --features rayon`;
- `cargo test -p rusty-matter-sdf --features parallel`;
- `cargo test -p rusty-matter-particles --features parallel`;
- `cargo test -p rusty-matter-surface-runtime --features parallel`;
- fixture validation;
- schema catalog export check;
- Matter dependency and namespace boundary scans.

For Matter batch execution work, the narrow test route is:

```powershell
cargo test -p rusty-matter-batch
cargo test -p rusty-matter-batch --features rayon
```

Those tests verify deterministic logical chunk construction and serial
chunk-index-ordered diagnostics reduction. With the `rayon` feature enabled,
they also verify local-pool Rayon execution, serial-vs-Rayon output
equivalence, batch-size-invariant integer diagnostics, and deterministic
chunk-index reduction order. The slice-chunk sentinel tests also cover varied
lengths and batch sizes to prove each output index is written exactly once and
no chunk writes outside its assigned range.

For general fixed-step particle execution work, the narrow test route is:

```powershell
cargo test -p rusty-matter-particles
cargo test -p rusty-matter-particles --features parallel
```

Those tests verify the serial default `ParticleSimulator` path,
batch-size-invariant serial output, execution diagnostics, and serial-vs-Rayon
equivalence for the opt-in `parallel` feature. The general simulator keeps
schema-rich `ParticleState` as the public model, but its fixed-step hot path
uses compact reusable scratch inputs instead of cloning particle identity and
schema strings each step. The Quest animated hand-mesh path uses
`SurfaceParticleRuntime`; validate that separately when its surface-particle
execution path changes.

For particle batch timing sweeps, use the Matter-owned JSONL example:

```powershell
cargo run -p rusty-matter-particles --example particle_batch_sweep -- --quick
cargo run -p rusty-matter-particles --features parallel --example particle_batch_sweep -- --quick
```

The output rows use schema `rusty.matter.particles.batch_sweep.v1` and report
workload, backend, worker cap, batch size, particle count, chunk count,
elapsed timing, closest-surface samples, surface-distance node/leaf/triangle
test totals, neighbor checks, affected/rejected particles, clamped particles,
and max speed. Use `--full`, `--counts`, `--batch-sizes`,
`--leaf-triangle-counts`, `--frames`, `--warmup-frames`, and `--workload` for
larger local measurements. This example is not part of `check_all.ps1` because
timing is machine-dependent evidence. For Quest SDF/particle tuning, prefer a
surface-only release sweep such as:

```powershell
cargo run --release -p rusty-matter-particles --features parallel --example particle_batch_sweep -- --counts 32768 --batch-sizes 256 --leaf-triangle-counts 4,8,16,32 --frames 4 --warmup-frames 1 --workload surface
```

For packed SDF grid and mesh-to-SDF reference work, the narrow test route is:

```powershell
cargo test -p rusty-matter-sdf
cargo test -p rusty-matter-sdf --features parallel
```

Those tests verify packed-grid validation, mesh-to-SDF construction, voxel
budget enforcement, build-report diagnostics, x-fastest linear/cell helper
round trips, explicit checked-vs-clamped nearest-neighbor sampling behavior,
finite nearest-gradient helpers, unsigned tetrahedron fixture coverage, damaged
mesh/config/grid inputs, and byte-identical serial-vs-batched dense SDF output.
With the `parallel` feature enabled, they also verify the Rayon-backed batched
dense SDF builder against the serial reference output and diagnostics.

For adaptive distance field reference work, the narrow test route is:

```powershell
cargo test -p rusty-matter-adf
cargo run -p rusty-matter-fixtures -- validate
```

Those tests verify ADF construction from Matter packed SDF grids, tolerance
collapse, cell-budget rejection, schema validation, and nearest leaf-cell
sampling. The fixture route validates
`fixtures/adf/unit-triangle-adaptive-field.json` and its compact summary so
future ADF acceleration work has a deterministic CPU reference artifact before
Quest or GPU adapters consume it.

For accelerated mesh-distance work, the narrow test route is:

```powershell
cargo test -p rusty-matter-mesh distance_sampler
```

Those tests verify exact closest-point behavior and that dense-surface queries
prune exact triangle tests through the Matter-owned sampler that browser Wasm
and Makepad/native adapters should share. They also verify that animated meshes
with unchanged topology can refit an existing distance-sampler tree without a
full rebuild, while changed topology is rejected rather than silently reusing an
invalid tree.

For native animated-surface runtime adapter work, the narrow test route is:

```powershell
cargo test -p rusty-matter-surface-runtime
cargo test -p rusty-matter-surface-runtime --features parallel
```

Those tests verify that the native facade updates animated surfaces, exposes
distance sampler diagnostics, probes the dynamic collider through the
batch-backed contact-probe path, accepts reusable contact-probe executors for
high-rate callers that should not rebuild Rayon pools per frame, builds an SDF
grid from the current surface,
steps Matter-owned surface particles, and refreshes browser-parity particle
distance snapshots without using the Wasm adapter. Particle distance refreshes
use the same low-rate particle execution settings and cached batch executor as
the particle runtime, while the high-rate distance array stays in Matter memory.
They also verify that consecutive matching-topology frames use the
distance-sampler refit path exposed on `MatterSurfaceRuntimeUpdate`. With the
`parallel` feature enabled, they additionally verify serial-vs-Rayon contact
probe output equivalence and serial-vs-Rayon particle-distance refresh output
equivalence. The default particle distance refresh policy remains the exact
snapshot path; tests also cover the explicit `StepOnly` policy used by headset
visual adapters to skip redundant pre-step snapshot refreshes without changing
particle integration samples.

For surface-field contract and dynamics work, the narrow test route is:

```powershell
cargo test -p rusty-matter-fields
```

Those tests verify field substrates over `MeshSurfaceSampleSet` nodes, scalar
and vector buffer validation, perturbation descriptor validation, runtime
config validation, sparse neighbor-plan construction, deterministic fixed-step
field dynamics, qualitative bioelectric circuit voltage/conductance/current
contracts, gated coupling, hysteresis memory, readout stepping,
realtime edit request/result/revision behavior, debug-sequence emission,
planarian AP circuit behavior over the reviewed GLB-derived educational mesh,
scenario outcome trace and trace-set contracts, normalized non-calibrated
planarian morphology/readout metric contracts, rights-safe species-like head
taxonomy contracts, tiny PlanformDB-derived review fixture contracts,
synthetic fallback coverage, and damaged neighbor/buffer/runtime/edit/trace/
mesh-anchor/source-evidence inputs.

The committed planarian mesh module is regenerated from a reviewed GLB with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Convert-PlanarianGlbSurface.ps1 `
  -GlbPath "<reviewed-planaria.glb>"
```

The converter verifies the reviewed SHA-256 by default, normalizes the mesh
into Matter coordinates, and writes
`crates\rusty-matter-fields\src\planarian_mesh_asset.rs`. The generated data is
a `TriangleMeshSurface` source, not a renderer asset or runtime GLB loader.

The committed dynamic surface-field and planarian bioelectric scenario fixtures
plus planarian outcome-trace, comparison-trace-set, normalized morphology
metric, species-like head taxonomy, PlanformDB-derived review, and
damaged-input fixtures are regenerated with the normal fixture route:

```powershell
cargo run -p rusty-matter-fixtures -- write
```

To build the browser WebAssembly adapter over the same Matter sampler:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-HandMeshWasmRuntime.ps1
```

The script installs `wasm-bindgen` under `target\wasm-tools` when missing,
builds `rusty-matter-handmesh-wasm` for `wasm32-unknown-unknown`, and writes
web-ready JS/Wasm artifacts under `local-artifacts\handmesh-wasm`.

To build the realtime surface-field and planarian bioelectric WebAssembly
adapter:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-SurfaceFieldWasmRuntime.ps1
```

The script builds `rusty-matter-fields-wasm` for `wasm32-unknown-unknown` and
writes web-ready JS/Wasm artifacts under `local-artifacts\surface-field-wasm`.
The exported package includes the Matter-owned surface-field runtime and the
Matter-owned planarian bioelectric runtime/edit surface. The Planarian 3D Wasm
surface also exports scenario reset codes for baseline, wound, gap-block,
transient-memory, and no-memory control presets over the reviewed GLB-derived
body substrate. The realtime preset uses 160 sampled GLB surface nodes with
five first-tier and five second-tier neighbor links per node, and exports
`node_surface_anchors()` rows as `[triangle_index, barycentric_a,
barycentric_b, barycentric_c]`. It also exports `node_activity()` rows as
`[absolute_voltage_delta, normalized_voltage_delta]` from the latest Matter
step or accepted voltage-changing edit so renderers can display realtime
activity without deriving circuit deltas from geometry. It also exports
`node_voltage_neighborhood()` rows as `[node_index, tier, weight]` so renderers
can preview the exact tiered voltage-brush targets before requesting the
matching `add_node_voltage_neighborhood()` mutation.
It also exports
deterministic outcome traces for renderer-side comparison plots, including the
Matter-owned comparison trace set used by Optics browser overlays and selected
node/edge readout accessors used by browser inspector panels. It also exposes
a bounded recent edit-event history for browser live feedback, plus affected
node/edge target rows for renderer highlights; browsers format and draw those
rows but do not own the event semantics.

## External Hand Mesh Validation

Matter's generic mesh contracts should also be exercised on a real deforming
hand mesh when an external hand-recording bundle is available. Keep those
captures and GLB files outside this repo.

The validation shape is:

```powershell
$BundleRoot = "<unzipped-handmesh-pipeline-bundle>"
$CaptureDir = "<pulled-hand-recordings-capture-dir>"
$OutputRoot = "<temporary-output-root>"

& "$BundleRoot\cli\Run-HandMeshPipeline.ps1" `
  -Command export-glb `
  -CaptureDir $CaptureDir `
  -RequiredHands both `
  -OutputRoot $OutputRoot
```

Use the exported GLB and pulled validation-mesh frames as external fixtures for
mesh import/export scripts. They prove the same topology/sample/collider/SDF
path on PC with a deforming Meta hand mesh, but the generic Matter mesh crate
must not become restricted to that hand topology.

To smoke-test an already exported GLB without headset access:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Invoke-HandMeshGlbSmoke.ps1 `
  -GlbPath "<exported-hand-mesh.glb>"
```

This writes `local-artifacts\hand-mesh-glb-smoke\summary.json` plus one
`TriangleMeshSurface` JSON file per supported triangle primitive. The command
uses a dependency-free GLB reader in `tools\extract_glb_mesh_surfaces.py`; it is
not part of `check_all.ps1` because the GLB is an external local artifact.

To test realtime deformation without headset access, extract an animated mesh
surface sequence from the same GLB:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Invoke-HandMeshGlbSequenceSmoke.ps1 `
  -GlbPath "<exported-hand-mesh.glb>" `
  -Output "<output-hand-mesh-sequence.json>" `
  -MeshIndex 0 `
  -FrameCount 120
```

The sequence contains sampled skinned vertex positions and shared triangle
topology only. It intentionally does not contain precomputed SDF grids,
collider frames, or particles; those must be recomputed by the realtime
simulation/preview path from the current mesh frame.

Slow cross-repo validation should run only before a larger bundle push.
