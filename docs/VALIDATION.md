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

Slow cross-repo validation should run only before a larger bundle push.
