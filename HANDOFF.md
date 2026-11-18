# Handoff

Last updated 2026-11-18, Claude. Merged on main: everything through #103. Open, all mine:
- #104 fix: Rust 1.100 (stable since 2026-11-09) flags a loop in tests/math_core.rs, so main's CI is red until it
  merges. Test-only; its CI passed on 1.100. Merge first.
- #105 docs: real-scan check of the clipping fix (two public research scans, local only) and the clipping finding
  closed; this file. Red only on the #104 lint.
- #106 io.gltf: glTF 2.0 / GLB reader (phone-scan export). Carries #104's commit so its CI ran the full gate on
  1.100: green. Checked on the Khronos sample models (cloned locally from KhronosGroup/glTF-Sample-Assets, not
  committed) and 3,000 corrupted GLBs without a crash.

## Open right now
- No open findings.
- Real-scan clipping table is in docs/findings/2026-11-07-clipping-on-real-scans.md. A fit that stops at the round
  cap cannot report whether one more round would change it; noted, not a bug.
- glTF with buffers in a separate `.bin` refuses (the reader gets bytes, not a folder). Reading the sibling file in
  the CLI/MCP loaders is the obvious follow-up if real phone exports need it.
- Build output: `/tmp` can be a small tmpfs. Put build output under `$HOME/.cache`.

## Read first
INTENT.md (locked; changes only by a superseding decision), AGENTS.md (outranks the rest), docs/v1.md (what v1 means,
phases P0–P5), docs/where-to-help.md (how to pick work). The roadmap is a map, not a limit (D-013).

## Find something to work on
`python3 tools/plan.py needs`, pick what helps the app most for its size, say what and why in one line, claim it with a
draft PR (feature id, or an area tag: fix:, improve:, perf:, test:, docs:), work in your own git worktree.

## Where things stand
- Engine (library): readers STL/PLY/OBJ/3MF; plane/sphere/cylinder fits plus robust versions; relations; thickness;
  segmentation (planes, cylinders, joined strips); deviation and patches; ICP and datum frame; hole patterns (line,
  circle, grid), symmetry, design-value guesses; CAD recipe and coverage check (`cad::check`); threads (`par::map`).
- Faces: CLI (info, fit, segment, project, thickness, deviation, version), MCP (open, info, select, fit incl. robust,
  measure, segment, compare, reverse, datum, thickness, undo/redo, guide), browser app prototype `rulercraft-gui`.
- Break-it pass 2026-11-12 (CLI): truncated/oversized/NaN/huge-coordinate STL, binary STL starting "solid", bad OBJ
  indices, lying PLY counts, NaN/inf in every selection option, bad units, tolerances and limits all refuse plainly with
  exit code 2. A part moved 1 km from the origin still fits its 3 mm hole within 1e-4 mm, with sigma growing honestly
  from f32 storage. Only cosmetic leftovers: "--sigma needs 1 numbers" (plural), `--near ... inf` says the radius
  "must be zero or a positive number"; a repeated option silently takes the last value.
- Known paperwork nits, left alone because DECISIONS.md is never rewritten: two decisions are numbered D-009 (3MF reader
  and GUI). ROADMAP.md has sections R1–R7; R8 (verification) lives only in plan/roadmap.json.

## Next, if nobody says otherwise
- R1 exit is the most valuable thing on the board and needs Logan's hands: print fixtures/known/hole-block.stl, caliper
  it, scan it, fit the scan. Engine work has run ahead of real-part evidence.
- P0 of docs/v1.md: native app shell (`app.shell`) and wgpu viewport with SolidCraft's theme (SolidCraft: ~/solidcraft,
  MIT OR Apache-2.0, egui/eframe 0.36, wgpu 30, theme in crates/ui-egui/src/theme.rs), then parity with the prototype.
- Verification without more CI: `verify.regress` (R8) as an ordinary test file in the existing gate.

## Owed and waiting on Logan
- R1 exit: a real part whose design or published dimensions match (above). Private scan re-run for #101.
- cad.solidcraft live replay and R2 exit need SolidCraft running. cad.check's exact version needs SolidCore.
- Publishing (installers, crates.io), Wasm toolchain (`rustup` is not installed), signing and a name check
  (docs/distribution.md).

## Habits that held up
- Never switch branches in a shared checkout; `git worktree add /tmp/<name> -b <branch> origin/main`, remove after merge.
- Every rule gets a wrong-value run a test fails on, or the rule goes. Undo a wrong-value edit with the reverse edit, not
  `git checkout <file>`: that also throws away the uncommitted fix (happened 2026-11-12, caught by grep).
- Two checkouts sharing one CARGO_TARGET_DIR can hand you the other checkout's binary (cargo kept the old build;
  `cmp` showed identical files). Give each checkout its own target dir before comparing behaviour.
- Merge commits only; wait for CI on the exact head commit before merging; on generated-file conflicts merge main and
  `python3 tools/plan.py render`; take main's HANDOFF.
- Never name other apps in the repo.
- Local only, never committed: fixtures/private (vase scan), local/NOTES.md.
