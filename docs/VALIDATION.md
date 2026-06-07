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

Slow cross-repo validation should run only before a larger bundle push.
