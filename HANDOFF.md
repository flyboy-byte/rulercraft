# Handoff

Last updated 2026-11-12, Claude. Merged on main: everything through #99. Open, all mine, independent of each other:
#100 (fix: a test-only clippy lint that fails on Rust 1.97), #101 (fix: sigma clipping settles; decides the clipping
finding, option b), #102 (fix: zero-area reference triangles left out of deviation; closes that finding), #103 (docs:
newcomer pass, changelog lines for #101/#102, this file). #101 and #102 are marked draft only because the session could
not edit PR state; their code and tests are complete and their reports are in this file's "Open right now".

## Open right now
- #101: the old loop recomputed the cut-off from each round's sigma, so heavy-tailed residuals crept to the 10-round cap;
  at k = 1 on 7,200 Laplace plane points it kept 88 (5,438 is right). Now the cut-off is k × the all-points sigma, fixed.
  Tests: tests/clip_settles.rs (failed on main). The CLI clips at 3 sigma by default, so default fits on scans change:
  more points kept, larger sigma and form error. Still owed: re-run the private scan's base face (5,969 points; expect
  more than 5,469 used, under 10 rounds) and close the finding with those numbers.
- #102: tests/deviation.rs adds an all-collinear reference (refused both sides) and a collinear triangle on a valid
  face's edge (below the face read +1, now -1). `Reference::new` also feeds ICP and the recipe check, which now refuse an
  all-zero-area reference too. Near-zero slivers keep their roundoff normal; not addressed.
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
- Merge commits only; wait for CI on the exact head commit before merging; on generated-file conflicts merge main and
  `python3 tools/plan.py render`; take main's HANDOFF.
- Never name other apps in the repo.
- Local only, never committed: fixtures/private (vase scan), local/NOTES.md.
