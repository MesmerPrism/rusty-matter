# Third-Party Notices

## Planaria Educational Mesh

Source: Sketchfab model "Planaria" by aphanizomenon.

Use in this repo: the reviewed GLB was translated into Matter
`TriangleMeshSurface` data in
`crates/rusty-matter-fields/src/planarian_mesh_asset.rs` and is included in the
planarian bioelectric fixture as an educational body surface. The bioelectric
dynamics over this body are qualitative synthetic presets, not measured
physiology, wet-lab data, or mechanistic prediction output.

License: CC-BY-4.0.

Attribution: aphanizomenon, Planaria, Sketchfab.

Source page:
https://sketchfab.com/3d-models/planaria-8e5a7c4e312e4b08b20676608cb2399f

License page: https://creativecommons.org/licenses/by/4.0/

Reviewed source GLB SHA-256:
a170a62ba705a81e73dd7fcfb5808431ff1a0b5c0da6322742c1e2c6ce480dda

Translated Matter surface stats: 13,663 vertices; 23,468 triangles.

Conversion command shape:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Convert-PlanarianGlbSurface.ps1 `
  -GlbPath "<reviewed-planaria.glb>"
```
