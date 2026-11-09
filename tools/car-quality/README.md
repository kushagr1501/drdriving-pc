# Detailed car pack

Run commands from the repository root. The generated pack is already available locally in
`web/car-pack/`; install it with `./Install-Detailed-Cars.ps1` and restart
the game. Original `.car` packages remain available as a fallback. See CREDITS.md for attribution.

To rebuild, install the pinned Node dependencies with:

```powershell
npm ci --prefix tools/car-quality
node tools/car-quality/export-hq-cars.mjs
node tools/car-quality/validate-pack.mjs
./Install-Detailed-Cars.ps1
```

The exporter reads the locally downloaded source GLBs and wheel-rig metadata under
`local-test-assets/modern-cars/<id>/` and `local-test-assets/bmw-320i/`. These large local inputs
are excluded from Git. Their download filenames, authors and source URLs are in `cars.json`
and CREDITS.md; the BMW source is `assets/bmw-320i.glb` in the same upstream asset repository.
Wheel pivots come from each source folder's `wheel-rig.json`.
The exporter updates both `web/car-pack/` and the local validation copies.

No mesh simplification or artificial texture upscaling is performed. Geometry is transformed,
wheel components separated, tangents generated, and nodes sharing a material and wheel group
merged to reduce draw calls. Paint normals use area-weighted smoothing across matching
positions, avoiding thin door-edge bevels distorting large panels. Original texture maps are
preserved separately as PNGs; paint/glass/tyre material tuning is recorded in the package.

The native package is `DRHQ0001`, a little-endian JSON-header length, a JSON manifest, then
binary image/vertex/index buffers. Vertices contain position, normal, UV and tangent with
handedness. The runtime keeps indexed mesh chunks below Macroquad's draw-call limit.

`prepare-reflections.mjs` regenerates the two reflection PNGs from the downloaded Poly Haven
panorama in `local-test-assets/quality-upgrade/`. The blur is an approximation of rough
reflections; this is not a full HDR lighting or ray tracing implementation. The source GLBs
also differ considerably in mesh and texture quality. Fine details absent from a source model
cannot be recovered by this renderer.
