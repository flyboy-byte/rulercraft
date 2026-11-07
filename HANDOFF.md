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
