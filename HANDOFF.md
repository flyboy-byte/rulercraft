# Handoff

Last updated 2026-11-18 (afternoon), Claude. Merged on main: everything through #107. Open, both mine:
- #108 cad.solidcraft: `solidcraft::design` and `rulercraft solidcraft` rebuild a scanned plate as an editable
  SolidCraft design (box, cut holes, named sizes, loss report). Checked in SolidCraft's own CLI. Merge first.
- #109 D-014 (approved by Logan): the SolidCraft importer plugin in `plugin/solidcraft` (own workspace, the only
  `wit-bindgen` user). Contains #108's commits until #108 merges. Its component passes SolidCraft's
  `plugin check` and all four `plugin test` cases, and a real install and import (docs/specs/solidcraft-plugin.md).

## Open right now
- No open findings.
- Plugin limits: plate-like parts only; bosses, slanted faces, tilted holes and non-rectangular outlines go to
  the import report. Next real step is a scan of a printed plate (R1/R2).
- `--facing` counts a vertex when any touching triangle faces the direction (#110, okayed by Logan); on scans
  this adds the faced surface's edge vertices (table in #110).
- Building SolidCraft here: pin is Rust 1.99.1 (unavailable offline); `RUSTUP_TOOLCHAIN=stable` (1.97, its
  rust-version is 1.95), `-j 1`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_DEV_PACKAGE_SOLIDCORE_OPT_LEVEL=0`
  or the 8 GB box runs out of memory; about 70 minutes, `--no-default-features` keeps plugin support.
- Building the plugin here without the wasm target: fetch `library/` (and the `library/backtrace` submodule) of
  rust-lang/rust at `rustc -vV`'s commit, link it to `$(rustc --print sysroot)/lib/rustlib/src/rust/`, then
  `RUSTC_BOOTSTRAP=1 cargo build --release --target wasm32-unknown-unknown -Z build-std=std,panic_abort`.
  CI's `solidcraft-plugin` workflow does it the normal way; its artifact download is blocked from this sandbox.
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
- v1 is delivered inside SolidCraft (D-015): next in docs/v1.md order, `sc.outline`, `sc.boss`, `sc.steps` (each
  checked in SolidCraft's own CLI as #108 was), then `sc.check`, `sc.inspect`, `sc.agent`.
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
