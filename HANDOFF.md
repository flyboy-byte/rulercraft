# Handoff

Last updated 2026-11-21, Codex. Main includes the assessment, roadmap and assisted rebuild through #142.

## Public plugin release (2026-11-22)

[Plugin 0.1.0](https://github.com/flyboy-byte/rulercraft/releases/tag/plugin-v0.1.0) is published on
tagged commit `e6b6fba`. The host gate passed 16 cases. The public ZIP checksum matched, and the
downloaded ZIP installed and edited the hole block through the Linux host. A user confirmed the
Model ribbon entry on desktop before this release. Native assisted rebuild, organic reconstruction,
physical scan accuracy and wider platform and host-version coverage remain open.

## Where the plugin stands
The SolidCraft importer (D-014, D-015) rebuilds a plate-like scan as an editable SolidCraft design: any straight or
circular outline (sketch extrude), through and blind holes, counterbores, bosses, pockets, steps and pads, through
windows, every size a named parameter (holes and bosses on the top follow `thickness`), a loss report of what was
dropped or simplified, a point-by-point deviation of the scan from the rebuilt part (`details.check`), and a
volume comparison made in SolidCraft (`details.volume_check`). Checked against SolidCraft-made parts: hole block,
bracket, L-plate, round plate, stepped cylinder, counterbore, window, countersink, two chamfered and two filleted plates
(all exact), 14/14 in SolidCraft's `plugin test`,
and driven over SolidCraft's MCP by `tools/agent_smoke.py` (docs/agents-and-solidcraft.md).

## Scans (2026-11-19)
No real scan has been tried yet, so scan-like meshes stood in: marching cubes over SolidCraft-made parts (a local
Python script with numpy, scipy and scikit-image, not committed: tools are Python without packages) and, in the
repo, `tests/scan_like.rs` (surface nets over an exact part, turned, with noise). They found and fixed: quadratic
round-region search (minutes), a frame turned by a ragged base boundary, sizes short by the softened edges, faces
split by noise, and hundreds of report items for softened edges. On the bracket made scan-like at 0.4 mm with 0.03 mm
noise, turned 23°: every size within 0.01 mm, the scan within 0.06 mm (p95) of the rebuild, 3 s. Outlines (L, round),
windows, countersinks, chamfers and fillets work on scans too now; every synthetic scan of the known parts rebuilds
within 0.25 % of SolidCraft's volume. `tests/ply_memory.rs` failed once under a full parallel run (its child's 96 MB
virtual-memory cap) and passed on every rerun: a flake to watch, not caused by this work.

## Scan poses (#129, 2026-11-19)
The Codex assessment (#127 item 3) found the scan rebuild failed at some turns. Fixed: pockets aligned by their walls,
softened corners and noise spikes dropped from snapped outlines (`snap_poly`), windows on scans rebuilt from their
walls (`wall_poly`, falls back to snapping), boundary tracing that takes the sharpest turn at junctions, and, on scans
only, a chamfer must be larger than half of `soft` (1.5 edge lengths): smaller is the scan's own edge softening, which
at 0° lines up with the grid and runs the whole edge. Swept by hand at 30 turns, all pass:
`for a in $(seq 0 15 345); do RULERCRAFT_SCAN_TURN=$a cargo test -q --release -p rulercraft-mesh --test scan_like; done`.
The fuzz fit check now finds every non-finite number by path (only the optional diameters may be null). The wasm
plugin was not rebuilt locally (the proxy blocked the rustup download); CI builds it. Debug prints left in
`scan_like.rs` and `invariance.rs` were removed.

2026-11-19 Codex follow-up: #131 (`codex/scan-volume-validity`) keeps the rebuilt SolidCraft volume
when the scan is open and leaves the scan volume and difference unavailable. The baseline open-window
reproduction reported 8,180 mm³ for the scan and 1.22% difference from the 8,280 mm³ rebuild.
Recheck the closed and open window imports first.

## Also new (2026-11-19)
- Point clouds (#123): `fit` and `measure` read .xyz/.pts/.asc and points-only PLY (`read_cloud`). Not yet: E57/LAS/PCD,
  normals for clouds (so `--facing` works).
- Fuzzing (#124): `tests/fuzz.rs`, every reader and the fit command layer; long runs by hand. Not yet: the MCP server
  over stdio, coverage-guided fuzzing (a library: Logan), a scheduled T2 run.
- Test tools may use numpy, SciPy and scikit-image (D-016); `tools/dev/synth_scan.py` makes scan-like meshes.

## Owed and waiting on Logan
- R1/S1: print fixtures/known/hole-block.stl (or any plate), caliper it, scan it, open the scan with the plugin.
  Nothing here has met a real scan's noise yet; the rebuild's tolerances are guesses until it does.
- Try the install screen and Plugins menu in the SolidCraft desktop app; only its CLI and headless MCP were used.
- Later: crates.io publishing (a yes), signing, a name check (docs/distribution.md).

## Not done (ordered)
- `sc.inspect`: measuring inside SolidCraft. SolidCraft's `model.mesh` stops at 8,192 vertices for command plugins;
  needs a way around inside its interface, or a request to its maintainers (Logan's call).
- Sloped faces (countersinks, top-edge chamfers and fillets are done): bevels and rounds under the plate, tilted holes, mixed arc and line outlines (now straight-edge runs),
  blind or stepped cutouts. All reach the loss report as `dropped` or `degraded`, none are silent.
- Real-scan hardening: noise, open meshes, thin walls, scans not aligned to a face.
- Other roadmap items from `python3 tools/plan.py needs` (point clouds, cone/torus fits, mesh clean-up, sections).

## Assisted rebuild (2026-11-21)

- #134–#142 are merged. MCP opens and calibrates clouds, returns picked coordinates, and drafts an
  editable plate with one hole. The tool records each dimension's source. See the
  [walkthrough](docs/agents-and-solidcraft.md#rebuild-a-plate-from-a-point-cloud) and [spec](docs/specs/assisted-rebuild.md).
- On 12 noisy captures, opposing face fits reduced the largest width error from 0.676 mm to
  0.396 mm. A combined noisy draft matched SolidCraft volume before and after a thickness edit.
  The fitted angular coverage of a bore can still overstate actual wall visibility when off-wall
  samples are selected.
- Next are `sc.assisted` for native integration and `verify.phone` for an untouched physical phone
  export with held-out caliper readings.

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
  measure, segment, compare, reverse, datum, thickness, assisted rebuild, undo/redo, guide), browser prototype `rulercraft-gui`.
- Break-it pass 2026-11-12 (CLI): truncated/oversized/NaN/huge-coordinate STL, binary STL starting "solid", bad OBJ
  indices, lying PLY counts, NaN/inf in every selection option, bad units, tolerances and limits all refuse plainly with
  exit code 2. A part moved 1 km from the origin still fits its 3 mm hole within 1e-4 mm, with sigma growing honestly
  from f32 storage. Only cosmetic leftovers: "--sigma needs 1 numbers" (plural), `--near ... inf` says the radius
  "must be zero or a positive number"; a repeated option silently takes the last value.
- Known paperwork nits, left alone because DECISIONS.md is never rewritten: two decisions are numbered D-009 (3MF reader
  and GUI).

2026-11-19 Codex follow-up: #132 (`codex/bound-deviation-sampling`) caps deviation at 200,000
samples spread across the input sequence. The regression in `solidcraft_design.rs` reported
399,999 points before the fix. Review the count alongside the existing deviation checks.

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
