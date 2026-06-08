# Validation

Run the narrow repo-local check before committing:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

The check covers:

- `cargo fmt --all --check`;
- `cargo test --workspace`;
- fixture validation;
- schema catalog export check;
- Matter dependency and namespace boundary scans.

For accelerated mesh-distance work, the narrow test route is:

```powershell
cargo test -p rusty-matter-mesh distance_sampler
```

Those tests verify exact closest-point behavior and that dense-surface queries
prune exact triangle tests through the Matter-owned sampler that browser Wasm
and Makepad/native adapters should share.

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
scenario outcome trace contracts, synthetic fallback coverage, and damaged
neighbor/buffer/runtime/edit/trace inputs.

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
plus planarian outcome-trace fixtures are regenerated with the normal fixture
route:

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
body substrate, plus deterministic outcome traces for renderer-side comparison
plots.

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
