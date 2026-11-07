# Implementation plan

Last updated 2026-11-07. How the roadmap (ROADMAP.md) gets built: the order, what can run in parallel,
what each feature's PR contains and roughly how big it is. Live state (claimed, done) is
`python3 tools/plan.py next` and STATUS.md; this file is the map. Update it in the PR that changes the
order or adds a feature.

Sizes are for one agent session: **S** under an hour, **M** one to two hours, **L** split it first.
Every feature PR follows PROCESS.md: spec first if missing, checks first, one wrong-value run, the
report.

## Where R1 stands (2026-11-07)

Merged: STL/PLY/OBJ/3MF, mesh graph, selection, all three fits, relations, command layer, MCP,
plane/cylinder/strip segmentation, known-design corpus, project JSON and local reference checks.
Undo/redo remains planned. Segmentation, projects and thickness have no MCP tools yet; thickness is
an engine-only R3 API. R1's published-dimension/printed-part acceptance is still open. The integration
checkpoint found selection/axis edge cases and a separate math-range follow-up; see docs/findings/.

## R1: clean meshes, open, fit, measure, over CLI and MCP

**Exit:** Logan downloads an STL, asks Claude over MCP for its hole diameters and face distances, and
they match the design dimensions.

### Critical path (in order)

| # | Feature | Size | What the PR contains | Needs |
| --- | --- | --- | --- | --- |
| 1 | ~~`fit.cylinder`~~ (done 2026-11-07) | M | `fit/cylinder.rs`: axis from normals, circle fit, LM refinement, clipping, coverage; tests on full tube, 120° arc, tilted axis, flat refusal, an optimality check like the sphere's | math.core, mesh.graph |
| 2 | ~~`cli.commands`~~ (done 2026-11-07) | M | A small command layer in `rulercraft-mesh::commands` (one typed input/output per command: `mesh.open`, `select.box`, `select.facing`, `fit.plane`, `fit.sphere`, `fit.cylinder`), and `rulercraft fit <kind> <mesh> --box ... [--facing x y z --within deg] [--sigma k]` printing JSON. Tests run the binary on generated meshes | the fits |
| 3 | ~~`measure.relations`~~ (done 2026-11-07) | M | `measure.rs`: plane-plane angle and distance, point/centre-plane distance, centre-centre, axis-plane angle, axis-axis angle and distance (parallel case), each saying which convention applied; `rulercraft measure` | fits |
| 4 | ~~`mcp.server`~~ (done 2026-11-07) | M | New crate `rulercraft-mcp` (official Rust MCP SDK is pre-approved, D-006): stdio server, one tool per command with its JSON schema, a short built-in guide; a test that drives it over stdio | cli.commands |
| 5a | ~~`segment.planes`~~ (done 2026-11-07) | M | Region growing of near-coplanar neighbouring triangles into flat regions with their plane fits, largest first | mesh.graph, fit.plane |
| 5b | ~~`segment.cylinders`~~ (done 2026-11-07) | M | Round regions (holes, bosses) from how normals turn around an axis | 5a, fit.cylinder |
| 5c | ~~`segment.strips`~~ (done 2026-11-07) | S | Join the flat strips a faceted round surface breaks into | 5a, 5b |
| 6 | ~~`corpus.known`~~ (done 2026-11-07) | M | A generator (Rust test helper or script) that writes test parts with exact stated dimensions as STL plus `fixtures/known/<part>.json` answers: a block with 3/5/8 mm holes at stated spacing, a stepped cylinder, a sphere seat, a bracket with two parallel faces at a stated distance. Tests: every answer recovered within a stated tolerance | fits, measure |
| 7 | R1 exit run | S | Logan's check: an STL from a model-sharing site with published dimensions, driven over MCP. Record the result in ROADMAP.md (struck through with a date when it passes) | all above |

### Parallel lanes (any time)

| Feature | Size | Notes |
| --- | --- | --- |
| ~~`io.3mf`~~ (done 2026-11-07) | M | Zip + model XML; zip/deflate crate pre-approved (D-006); 3MF's own unit wins; components and transforms flattened |
| ~~`project.file`~~ (done 2026-11-07) | M | Own JSON format (docs/specs/project-file.md): version, embedded source mesh bytes and units, elements with their recipe (selection, settings) and last result; reopen refits and reports changes. serde pre-approved |
| `history.undo` | S | Once commands exist: every successful command one undo step, failed ones change nothing |
| `mcp.thickness` | S | Expose paged directional wall-thickness samples over MCP |
| ~~`reference.local`~~ (done 2026-11-07) | S | A test helper that loads `fixtures/private/reference/<mesh>.json` and skips with a printed reason when absent |
| ~~STL overflow fix~~ | S | DONE 2026-11-07: the shared mesh builder refuses a coordinate that overflows f32 after unit conversion, for every reader |

## Checkpoints (PROCESS.md "Checkpoints")

| Checkpoint | Due | Scope |
| --- | --- | --- |
| ~~`checkpoint.r1-fits`~~ | DONE 2026-11-07 (#19) | readers, graph, selection, all three fits, relations: invariance, hostile input, real scan |
| ~~`checkpoint.r1-mcp`~~ | DONE 2026-11-07 (#30) | drive the MCP server end to end on the real scan; invariance and hostile input for anything new; findings |
| `checkpoint.r1-exit` | #32 integration audit; milestone acceptance remains open | everything in R1 against the known-design corpus and downloaded models; then Logan's R1 exit run |
| `checkpoint.r2` / `r3` / ... | at each milestone exit | the same, for that milestone's features |

## R2: mesh to editable CAD through SolidCraft

**Exit:** a simple prismatic downloaded part is rebuilt as a SolidCraft model whose deviation from the
STL is under 0.05 mm.

1. **`cad.solidcraft` (L, split):** (a) a written mapping from RulerCraft elements to SolidCraft MCP
   operations (plane → face/sketch plane, cylinder → hole or boss, distances → dimensions), checked
   against SolidCraft's MCP tool list; (b) a skill/guide so Claude can drive both servers in one
   session; (c) a worked example in `docs/examples/` on a corpus part.
2. **`cad.check` (M):** export the rebuilt part as STL or STEP from SolidCraft, then mesh-to-mesh
   deviation (`compare.deviation`) gives the number. Direct SolidCore projection comes later, behind an
   optional feature (D-002).
3. **`compare.deviation` (M):** per-vertex signed distance to the closest point on a reference mesh
   (a bounding-volume tree for speed), statistics and over-tolerance patches.

## R3: noisy scans

**Exit:** a real scan gives stable fits and a deviation report against its CAD.

1. `fit.robust` (M): RANSAC seeding for plane/sphere/cylinder so far-off strays stop pulling fits
   (the limit recorded in the fit specs).
2. `align.icp` (M): datum alignment (three planes; plane-line-point) and point-to-plane ICP.
3. `measure.thickness` (M): wall thickness along the inward normal.
4. A noisy-scan corpus (local only: public sample scans, phone scans) with notes in `local/`.

## R4: reverse-engineering workflows with AI

1. `re.features` (L, split): hole patterns (equal diameters on a circle or grid), symmetry planes,
   snapping measured values to likely design values with the evidence for each guess.
2. A feature-tree proposal format Claude can present and a person can accept or edit, then send to
   SolidCraft (R2's mapping).

## R5: reusable engine

`engine.library` (M): a stable public API review, rustdoc examples, a Wasm build in CI, a crates.io
release when Logan says so.

## R6: photos to mesh (research)

`photo.mesh` (L, research): a written go/no-go first (what is feasible, which permissive libraries
exist, what quality a phone gives), then one prototype object. No code lands before the write-up.

## Rules of thumb for picking

- Take the lowest-numbered critical-path item that is ready, else any parallel lane.
- Two sessions never take the same feature: claim with a draft PR first, `plan.py next` shows claims.
- If a feature is L, the first PR splits it into feature files (a), (b), (c) and does (a).
