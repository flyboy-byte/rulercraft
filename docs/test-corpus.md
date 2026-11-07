# Test corpus

Committed meshes are generated in tests or designed by us, with their known answers. Downloaded
models and other programs' numbers stay in gitignored `fixtures/private/` with notes in gitignored
`local/` (DECISIONS.md D-004); tests that use them skip when they are absent.

| File | Source | Licence | Known dimensions | Used by |
| --- | --- | --- | --- | --- |
| (generated in tests) unit cube | `crates/rulercraft-mesh/tests` | ours | 1 x 1 x 1 mm, 8 vertices, 12 triangles | io.stl |

To add: our own test parts designed in SolidCraft with round, stated dimensions (a block with holes of
3, 5 and 8 mm at stated spacing, a stepped cylinder, a sphere seat), exported as STL and committed with
their design file; and a version Logan prints and measures with calipers.
