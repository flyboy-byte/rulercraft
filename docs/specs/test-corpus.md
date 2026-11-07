# Test corpus

Status: R1 spec (2026-11-07). Feature: `corpus.known`. Registry: docs/test-corpus.md.

- Commit four original, closed, consistently wound triangle meshes in `fixtures/known/`, licensed
  MIT OR Apache-2.0. `tools/generate_known.py` is their editable procedural design source; it uses
  only Python's standard library and writes binary STL in mm. No downloaded geometry.
- Block: 60 x 30 x 10 mm, through holes of diameter 3, 5 and 8 mm, centres (10,15), (30,15),
  (50,15), axes along z. Adjacent hole spacing 20 mm.
- Stepped cylinder: diameter 20 from z=0 to 10, diameter 12 from z=10 to 20. Total height 20 mm.
- Sphere seat: concave spherical surface of radius 12 centred at (0,0,12), from bottom z=0 to
  rim z=6, in a round body of diameter 30 from z=-3 to 6.
- Bracket: U cross section in x/z, outside width 40 and height 20, extruded 20 in y. Base thickness
  8, walls thickness 8; inner faces x=8 and x=32 are parallel and 24 mm apart.
- Curves are chordal meshes with sampled vertices on the exact design surface. Fits use the wall
  vertices, not polygon midpoints. JSON companions state bounds, recipes and fixed expected
  dimensions independently of engine results. Regeneration changes meshes, never expected answers.
- Check every stated answer through import, CLI fits and engine relations. Interior wall bands
  avoid shared cap/shoulder vertices. Default tolerance 1e-5 mm covers f32 quantization (under
  4e-6 mm at these coordinates) and solver roundoff; integer bounds are exact. No tolerance retuning.
  Check closed boundaries and no non-manifold edges. Keep this four-part check under a minute.
- These are mathematical designs, not manufacturing or scan references. Printing/caliper checks
  and downloaded parts with published dimensions remain a separate R1 exit activity.

Checkpoint.r1-exit (#32, 2026-11-07) repeats all 41 fixed corpus dimensions/relations through MCP,
checks the private scan against CLI and a prior local rounded reference, and extends bounded hostile
input tests to ZIP 3MF and project JSON. It does not close the milestone or substitute for published
dimensions and printing/caliper measurements.
