# Fixtures

Fixtures are deterministic evidence for Matter contracts and CPU reference
behavior.

The SDF foundation bundle currently includes:

- mesh payload fixtures in `fixtures/mesh`;
- dynamic mesh surface, sample, coordinate-map, and dynamic-collider summaries
  in `fixtures/mesh`;
- synthetic hand validation mesh frame fixtures in `fixtures/hand`;
- full packed SDF grid fixtures in `fixtures/sdf`;
- compact SDF summary goldens in `fixtures/sdf`;
- damaged-input rejection reports in `fixtures/damaged`;
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
the deterministic repo fixtures generic and small.
