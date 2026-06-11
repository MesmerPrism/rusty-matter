# Fixtures

Fixtures are deterministic evidence for Matter contracts and CPU reference
behavior.

The SDF foundation bundle currently includes:

- mesh payload fixtures in `fixtures/mesh`;
- dynamic mesh surface, sample, coordinate-map, and dynamic-collider summaries
  in `fixtures/mesh`;
- surface-field contract summaries, policy-free debug frames, and dynamic
  debug frame sequences in `fixtures/fields`;
- qualitative bioelectric circuit config, stepped state, and step diagnostics
  in `fixtures/fields`;
- bioelectric circuit edit request/result fixtures in `fixtures/fields`;
- planarian AP bioelectric scenario runs in `fixtures/fields`, including the
  reviewed GLB-derived educational source-surface provenance, substrate nodes,
  triangle/barycentric mesh anchors, mesh-normalized AP-region metadata,
  circuit debug frames, voltage, memory, readouts, and diagnostics;
- compact planarian AP outcome traces in `fixtures/fields`, including
  posterior memory/head readout, head/tail readout, cut-band voltage, and
  cross-cut conductance metrics derived from Matter scenario runs, plus the
  comparison trace set fixture that keeps baseline, wound, gap-block, memory,
  and no-memory traces on shared timing and metric columns;
- synthetic hand validation mesh frame fixtures in `fixtures/hand`;
- full packed SDF grid fixtures in `fixtures/sdf`;
- compact SDF summary goldens in `fixtures/sdf`;
- adaptive distance field fixtures and compact ADF summary goldens in
  `fixtures/adf`;
- damaged-input rejection reports in `fixtures/damaged`, including
  surface-field state and perturbation rejection cases;
- particle step summaries in `fixtures/particles`, including SDF attraction and
  richer interaction summaries.
- a render-neutral particle payload summary in `fixtures/particles`.

Regenerate fixtures with:

```powershell
cargo run -p rusty-matter-fixtures -- write
```

Validate fixtures with:

```powershell
cargo run -p rusty-matter-fixtures -- validate
```

External real hand-mesh captures and exported GLB files are intentionally not
committed here. Use them as local validation inputs for mesh scripts, then keep
the deterministic repo fixtures generic and small. The exception is the
reviewed Planaria educational GLB derivative in the planarian bioelectric
fixture and generated Rust module; it is committed as Matter triangle-surface
data with attribution in `THIRD_PARTY_NOTICES.md`.

`fixtures/fields/planarian-ap-comparison-outcome-trace-set.json` is the compact
comparison bundle for browser overlays and educational checks. It is generated
from Matter scenario runs, not from Optics or browser-side metric code.

For exported GLB captures, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Invoke-HandMeshGlbSmoke.ps1 `
  -GlbPath "<exported-hand-mesh.glb>"
```

Generated `local-artifacts` output is ignored by git.
