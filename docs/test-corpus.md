# Test corpus

Last updated 2026-11-07. Original meshes and fixed design answers are committed; downloaded models
and outside reference numbers stay in gitignored `fixtures/private/` with notes in `local/`.

| File | Editable design source | Licence | Known dimensions | Used by |
| --- | --- | --- | --- | --- |
| Generated unit cube | mesh tests | MIT OR Apache-2.0 | 1 x 1 x 1 mm; 8 vertices, 12 triangles | STL, PLY, OBJ import |
| [hole-block.stl](../fixtures/known/hole-block.stl) | [generate_known.py](../tools/generate_known.py) | MIT OR Apache-2.0 | 60 x 30 x 10 mm; holes diameter 3/5/8, 20 mm adjacent spacing | STL, cylinder fits, axis relations |
| [stepped-cylinder.stl](../fixtures/known/stepped-cylinder.stl) | same generator | MIT OR Apache-2.0 | diameters 20/12; two 10 mm steps, height 20; coaxial | STL, cylinder fits, axis relations |
| [sphere-seat.stl](../fixtures/known/sphere-seat.stl) | same generator | MIT OR Apache-2.0 | radius 12 at (0,0,12), rim z=6; body diameter 30, base z=-3 | STL, sphere fit |
| [parallel-bracket.stl](../fixtures/known/parallel-bracket.stl) | same generator | MIT OR Apache-2.0 | 40 x 20 x 20 mm U bracket; inner faces 24 apart; base/walls thickness 8 | STL, plane fits and distance |

Each STL has a companion JSON with the fixed bounds, selections, dimensions and tolerance reason.
The generator uses elementary lofts, fans and a polygon extrusion; no external geometry or copied
models. Run `python3 tools/generate_known.py` to edit/regenerate the meshes. It never rewrites answers.
Run `cargo test -p rulercraft-cli --test known_corpus` to check every stated answer through the real
CLI and engine relations; four closed meshes complete in about 0.14 seconds on the development machine.

Curves use sampled surface vertices with straight facets; these checks fit the vertices rather than
facet midpoints. The answers describe mathematical designs. Printed/caliper measurements and a
published-dimension downloaded part remain for the R1 exit check.
