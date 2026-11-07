# Architecture

Last updated 2026-11-06. How the code is laid out and the patterns every feature follows, so
sessions working in parallel build one system, not several. Change this file in the PR that
changes the pattern.

## Crates

| Crate | What lives there | Depends on |
| --- | --- | --- |
| `rulercraft-mesh` | The engine: readers, mesh model, selection, fits, measurements, project file. No I/O except reading bytes it is given. | nothing heavy (see Dependencies) |
| `rulercraft-cli` | The `rulercraft` binary: argument parsing, files, printing JSON. Thin. | mesh |
| `rulercraft-mcp` | Stdio MCP tools over the same commands; local file loading and JSON schemas. | mesh, rmcp |
| `rulercraft-cad` (to come, R2) | SolidCraft/SolidCore bridge, behind an optional feature. | mesh, SolidCore |

## Modules in `rulercraft-mesh`

```
mesh.rs        Mesh, MeshBuilder, Unit, Bounds            (exists)
stl.rs         STL reader                                  (exists)
io/            ply.rs, obj.rs, threemf.rs (exists), and a read_mesh(name, bytes, unit, limits) dispatcher
graph.rs       MeshGraph: normals, adjacency, components, boundaries, creases  (mesh.graph)
select.rs      box / near / facing / explicit selections   (select.region)
math/          small linear algebra: Vec3 helpers, 3x3 symmetric eigen, small least squares
fit/           plane.rs, sphere.rs, cylinder.rs, shared clipping loop
measure.rs     relations between elements                  (measure.relations)
segment.rs     automatic surface regions                   (segment.surfaces)
project.rs     project file                                (project.file)
commands.rs    the command layer: name -> typed input -> typed output, undo  (cli.commands, history.undo)
```

Move a module into its own crate only when it is large and stable.

## Patterns

- **Errors.** Each area has its own error enum with a `Display` message a user can act on (see
  `stl::ReadError`). No panics, `unwrap` or `expect` in library code; tests may unwrap.
- **Limits.** Anything driven by input size takes a limits struct with sensible defaults.
- **Units.** Millimetres and radians inside; the CLI/MCP edge converts degrees and other length units.
- **Precision.** Positions are `f32` (what files carry); all fitting and measuring math is `f64`.
- **Results are data.** Fit and measure results are plain structs with public fields, so the CLI, MCP and
  project file can serialise them without the engine knowing about JSON.
- **Commands** (when `cli.commands` grows): every user action is one named command (`mesh.open`,
  `fit.plane`, ...) with a typed input and output; the CLI and MCP only translate. One command, one undo
  step; a failed command changes nothing.
- **Tests.** Each feature has one test file in its crate's `tests/` (e.g. `tests/fit_plane.rs`) with its
  math checks; shared synthetic shapes (cube, tube, sphere cap, noisy plane) go in `tests/common/`.
  Optional local-data tests skip with a printed reason when `fixtures/private/` lacks the file.

## Dependencies (defaults, so nobody has to ask)

Decided in DECISIONS.md D-006. Default to no dependency. Pre-approved when a feature needs them:
`serde` + `serde_json` (project file, CLI/MCP JSON), a zip/deflate reader for 3MF, the official Rust MCP
SDK for `mcp.server`. Check each one's licence is MIT/Apache/BSD-style when adding it and note it in the
PR. Anything else gets a DECISIONS.md line first. Linear algebra is our own small `math/` module (a 3x3
symmetric eigen solver and small least squares are all R1 needs).

Import dispatch uses `io::ImportLimits` (also exported at the crate root) with byte, source-vertex
and candidate-triangle caps. The original STL-only `Limits` API remains available.

3MF uses zip for bounded in-memory package reads and a private XML geometry-subset reader. Its
model unit overrides the caller unit; build/component transforms flatten through MeshBuilder.
