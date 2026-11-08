# Handoff

Last updated 2026-11-07, Claude. Read AGENTS.md first (it outranks this), then PROCESS.md.

## State
- All feature PRs are merged; check `gh pr list` and `python3 tools/plan.py next` before starting.
- Shipped this day: readers (STL/PLY/OBJ/3MF), graph, selection, plane/sphere/cylinder fits, relations,
  thickness, CLI (`info`, `fit`, `segment`), MCP server (open, info, select, fit, measure, segment,
  compare, undo/redo, guide), project file, local reference numbers, known-design corpus,
  segmentation (planes, cylinders, joined strips), `math.range`, and the library pieces below.
- Library only so far (no CLI/MCP yet): `cad::recipe` (cad.solidcraft, `implemented`), `fit_plane_robust`
  (fit.robust, `verified`), `reverse::hole_patterns` and `design_guesses` (re.features,
  `implemented`), `align::datum_frame` (verified), `deviation::patches` (verified). `align_icp` and
  `deviation` are reachable through the MCP `compare` tool.

## v1 (D-012) — read docs/v1.md
- Logan chose: all four jobs, native desktop app (egui/eframe/egui-wgpu/wgpu at SolidCraft's versions, approved), SolidCraft
  community judges done, credit pacing open. Phases P0–P5 in docs/v1.md; P0 is next (native shell + viewport + parity).
- The browser viewer (rulercraft-gui) is now a prototype/test harness.

## Intent and scope (D-010)
- INTENT.md is locked (changes only by a superseding decision). We compete: milestone R7 and
  docs/specs/product-scope.md list the everyday-job gaps (constructions, tolerances, deviation map, alignment,
  sections, reports, projects, big meshes, browser build) in slice order. Overall % fell to ~61 because R7 is new.
- Never name the other apps in this repository; keep specs in our own words.

## Look, GPU, embedding (D-011)
- UI look comes from SolidCraft (MIT OR Apache-2.0, egui + wgpu, has gpu/render/mcp crates), never from a copyleft app.
- GPU is an accelerator verified against the CPU reference; native wgpu needs Logan's approval (new dependency).
- SolidCraft's plugin mechanism is undecided (its issue #1221 proposes sandboxed Wasm plugins calling `run(command, args)` JSON, same as MCP). Do `engine.wasm` + `engine.command-api`; stay host-agnostic (library, MCP, recipe JSON, postMessage page).
- Large-mesh plan: engine.bvh, engine.parallel, engine.stream, gui.lod, engine.gpu (product-scope.md).

## GUI (D-009)
- `rulercraft-gui` is the standalone app: std-only server plus hand-written WebGL (docs/specs/gui.md). First slice
  works (open, look, mark, fit, measure, find regions); it was checked by hand in headless Chrome and the server by
  tests. Next slices are listed in the spec's "Not done". Keep it a thin client over the engine and embeddable, so
  it can also be a panel in SolidCraft.

## Owed
- cad.solidcraft: one replay of a recipe against a live SolidCraft server (SolidCraft is not built on
  this machine; mind the disk guard before building it).
- cad.check needs SolidCore projection (optional cargo feature, D-002).
- re.features grid patterns; plane-line-point
  datums; CLI/MCP commands for recipe, robust fit, patterns and datum frame.
- R1 exit: Logan's run on a real downloaded STL (compare hole sizes with the real design), and the
  clipping decision (docs/findings/2026-11-07-clipping-on-real-scans.md).
- engine.library (R5) is in progress (docs, doctest, prelude, API page done); publishing needs Logan.

## Habits that held up
- Claim by draft PR titled `<feature id>: ...`; re-run `plan.py next` right before claiming.
- Every rule gets a wrong-value run that a test fails on, or the rule is deleted.
- Codex and Claude share one checkout: do your work in `git worktree add /tmp/<name>`, never switch
  branches in `~/projects/rulercraft`. A commit made there once landed on Codex's branch.
- Merge commits only. A merge conflict in generated files: merge main, `python3 tools/plan.py render`.
- Local only, never committed: fixtures/private (vase scan) and local/NOTES.md.
