# Fixtures

Fixtures are deterministic evidence for Matter contracts and CPU reference
behavior.

The SDF foundation bundle currently includes:

- mesh payload fixtures in `fixtures/mesh`;
- dynamic mesh surface, sample, coordinate-map, coordinate-map package, and
  dynamic-collider summaries in `fixtures/mesh`;
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
- normalized planarian morphology/readout metrics in `fixtures/fields`, with
  mesh-normalized head and pharyngeal extents plus a head-identity readout
  extent. These are educational summaries, not calibrated area measurements;
- a rights-safe species-like head-label taxonomy in `fixtures/fields`, using
  generated-symbol/text labels only and no paper figure reuse;
- a source-reviewed qualitative dynamics-target fixture in `fixtures/fields`,
  mapping high-confidence literature targets to current synthetic scenarios,
  future fixture gates, and explicit blocked calibration claims;
- a public Planarian XR neuron-cloud display-bridge fixture in
  `fixtures/fields`, preserving bridge/source-map/GLB/replay hashes and
  blocked observed-dynamics capabilities without becoming runtime dynamics;
- a request-only Planarian XR display-substrate graph policy in
  `fixtures/fields`, requesting one node per mapped public element and a
  deterministic nearest-neighbor display graph for later materialization;
- a small PlanformDB-derived review fixture in `fixtures/fields`, preserving
  source IDs, citation IDs, notice text, normalized labels, sample counts, and
  outcome frequencies without importing raw database rows or runtime dynamics;
- synthetic hand validation mesh frames and the provider-bound
  `hand-substrate-conformance.json` CPU-skinning oracle in `fixtures/hand`;
- full packed SDF grid fixtures in `fixtures/sdf`;
- compact SDF summary goldens in `fixtures/sdf`;
- adaptive distance field fixtures and compact ADF summary goldens in
  `fixtures/adf`;
- damaged-input rejection reports and hand provider-mixup/invalid-rig cases in
  `fixtures/damaged`, including
  surface-field cases and strict rejection of application, platform, renderer,
  private-driver, and high-rate-control particle-contract leakage;
- particle step summaries in `fixtures/particles`, including SDF attraction and
  richer interaction summaries.
- a render-neutral particle payload summary in `fixtures/particles`;
- `fixtures/particles/contract-conformance.json`, which binds the existing
  state, fixed-step, diagnostics, render-payload, and surface-snapshot schemas
  without adding an application-derived particle type.

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

`fixtures/fields/planarian-normalized-morphology-metrics.json`,
`fixtures/fields/planarian-species-like-head-taxonomy.json`,
`fixtures/fields/planarian-source-dynamics-targets.json`,
`fixtures/fields/planarian-xr-neuron-cloud-display-bridge-v0.json`,
`fixtures/fields/planarian-xr-neuron-cloud-display-substrate-request-v0.json`,
and
`fixtures/fields/planformdb-derived-v0.json` are source-target validation and
annotation artifacts. They do not change Matter's voltage, conductance,
memory/readout, or stepping behavior.

For exported GLB captures, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Invoke-HandMeshGlbSmoke.ps1 `
  -GlbPath "<exported-hand-mesh.glb>"
```

Generated `local-artifacts` output is ignored by git.
