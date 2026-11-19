# Handoff

Last updated 2026-11-18 (evening), Claude. Main is at #116; no open PRs.

## Where the plugin stands
The SolidCraft importer (D-014, D-015) rebuilds a plate-like scan as an editable SolidCraft design: any straight or
circular outline (sketch extrude), through and blind holes, counterbores, bosses, pockets, steps and pads, through
windows, every size a named parameter (holes and bosses on the top follow `thickness`), a loss report of what was
dropped or simplified, a point-by-point deviation of the scan from the rebuilt part (`details.check`), and a
volume comparison made in SolidCraft (`details.volume_check`). Checked against SolidCraft-made parts: hole block,
bracket, L-plate, round plate, stepped cylinder, counterbore, window, countersink, two chamfered plates (all
exact), 12/12 in SolidCraft's `plugin test`,
and driven over SolidCraft's MCP by `tools/agent_smoke.py` (docs/agents-and-solidcraft.md).

## Owed and waiting on Logan
- Push tag `plugin-v0.1.0` (or a newer commit's): `git tag plugin-v0.1.0 <sha> && git push origin plugin-v0.1.0`. A tag push
  is refused (403) from the Claude session's proxy; the `solidcraft-plugin-release` workflow does the rest.
- R1/S1: print fixtures/known/hole-block.stl (or any plate), caliper it, scan it, open the scan with the plugin.
  Nothing here has met a real scan's noise yet; the rebuild's tolerances are guesses until it does.
- Try the install screen and Plugins menu in the SolidCraft desktop app; only its CLI and headless MCP were used.
- Later: crates.io publishing (a yes), signing, a name check (docs/distribution.md).

## Not done (ordered)
- `sc.inspect`: measuring inside SolidCraft. SolidCraft's `model.mesh` stops at 8,192 vertices for command plugins;
  needs a way around inside its interface, or a request to its maintainers (Logan's call).
- Sloped faces (countersinks and top-edge chamfers are done): fillets, bevels under the plate, tilted holes, mixed arc and line outlines (now straight-edge runs),
  blind or stepped cutouts. All reach the loss report as `dropped` or `degraded`, none are silent.
- Real-scan hardening: noise, open meshes, thin walls, scans not aligned to a face.
- Other roadmap items from `python3 tools/plan.py needs` (point clouds, cone/torus fits, mesh clean-up, sections).

## Open right now
- No open findings.
- `--facing` counts a vertex when any touching triangle faces the direction (#110, okayed by Logan).
- Building SolidCraft here: pin is Rust 1.99.1 (unavailable offline); `RUSTUP_TOOLCHAIN=stable` (1.97, its
  rust-version is 1.95), `-j 1`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_DEV_PACKAGE_SOLIDCORE_OPT_LEVEL=0`
  or the 8 GB box runs out of memory; about 70 minutes, `--no-default-features` keeps plugin support.
- Building the plugin here without the wasm target: fetch `library/` (and the `library/backtrace` submodule) of
  rust-lang/rust at `rustc -vV`'s commit, link it to `$(rustc --print sysroot)/lib/rustlib/src/rust/`, then
  `RUSTC_BOOTSTRAP=1 cargo build --release --target wasm32-unknown-unknown -Z build-std=std,panic_abort` and
  `wasm-tools component new`; `plugin/solidcraft/build.sh` copies the fixtures into the package tests.
  CI's `solidcraft-plugin` workflow does it the normal way.
- Build output: `/tmp` can be a small tmpfs. Put build output under `$HOME/.cache`.

## Read first
INTENT.md (locked; changes only by a superseding decision), AGENTS.md (outranks the rest), docs/v1.md (what v1 means,
phases S0–S4), docs/where-to-help.md (how to pick work). The roadmap is a map, not a limit (D-013).

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

## Habits that held up
- A branch cut from `origin/main` goes stale once a PR merges: `git fetch` and rebase before building on it (a missed
  rebase cost a conflict on #116). Never switch branches in a shared checkout; `git worktree add /tmp/<name> -b <branch> origin/main`, remove after merge.
- Every rule gets a wrong-value run a test fails on, or the rule goes. Undo a wrong-value edit with the reverse edit, not
  `git checkout <file>`: that also throws away the uncommitted fix (happened 2026-11-12, caught by grep).
- Two checkouts sharing one CARGO_TARGET_DIR can hand you the other checkout's binary (cargo kept the old build;
  `cmp` showed identical files). Give each checkout its own target dir before comparing behaviour.
- `git add -A` on a branch that lacks a folder's `.gitignore` sweeps in build output left in the working tree (it
  put plugin/solidcraft's built .wasm and copied fixtures on main via #110; untracked in #112). Check `git status`.
- Merge commits only; wait for CI on the exact head commit before merging; on generated-file conflicts merge main and
  `python3 tools/plan.py render`; take main's HANDOFF.
- Never name other apps in the repo.
- Local only, never committed: fixtures/private (vase scan), local/NOTES.md.
