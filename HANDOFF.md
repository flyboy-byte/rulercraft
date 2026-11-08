# Handoff

Last updated 2026-11-07, Codex. Main includes #81–#83 and #85–#94: hole-pattern validation, bounded and faster imports, CLI thickness and deviation, cross-interface plane verification, large-range pattern handling, version reporting, and closed boundary-loop filtering.

## Open right now
- #95 (Codex, ready): compare signed plane deviation across the library, CLI and MCP; branch `codex/deviation-agreement`, worktree `/tmp/rulercraft-deviation-agreement`. Full workspace tests, format, clippy and plan checks pass; deliberate wrong mean values caused the CLI and MCP checks to fail. Next: check hosted CI and review the current head.
- #73 (Claude, draft): engine.parallel. `par::map` is bit-identical for any thread count and used by deviation, ICP, symmetry. Its speed test was still below target; check the PR for current state before touching related files.
- Build output: `/tmp` is a 7.5 GB tmpfs and filled up once. Put build output in a separate directory under `$HOME/.cache`.

## Read first
INTENT.md (locked; changes only by a superseding decision), AGENTS.md (outranks the rest), docs/v1.md (what v1 means,
phases P0–P5), docs/where-to-help.md (how to pick work). The roadmap is a map, not a limit (D-013).

## Find something to work on
`python3 tools/plan.py needs`, pick what helps the app most for its size, say what and why in one line, claim it with a
draft PR (feature id, or an area tag: fix:, improve:, perf:, test:, docs:), work in your own git worktree.

## Where things stand
- Engine (library): readers STL/PLY/OBJ/3MF; plane/sphere/cylinder fits plus robust versions; relations; thickness;
  segmentation (planes, cylinders, joined strips); deviation and patches; ICP and datum frame; hole patterns (line,
  circle, grid), symmetry, design-value guesses; CAD recipe and coverage check (`cad::check`).
- Faces: CLI (info, fit, segment, project, thickness, deviation, version), MCP (open, info, select, fit incl. robust, measure, segment, compare,
  reverse, datum, thickness, undo/redo, guide), browser app prototype `rulercraft-gui` (D-009; now a prototype per D-012).
- Decisions today: D-009 GUI, D-010 intent locked, D-011 SolidCraft look / GPU as verified accelerator / host-agnostic
  embedding, D-012 v1 = all four jobs, native egui+wgpu app (deps approved for app crates only), SolidCraft community
  judges done, D-013 open improvement + verification at scale.
- Docs added today: INTENT.md, docs/v1.md, docs/specs/product-scope.md, docs/specs/gui.md, docs/specs/verification.md,
  docs/specs/scan-sources.md, docs/distribution.md, docs/where-to-help.md, CHANGELOG.md, issue forms.

## Next, if nobody says otherwise
- P0 of docs/v1.md: native app shell (`app.shell`) and wgpu viewport with SolidCraft's theme (SolidCraft: ~/solidcraft,
  MIT OR Apache-2.0, egui/eframe 0.36, wgpu 30, theme in crates/ui-egui/src/theme.rs), then parity with the prototype.
- Engine: the shared spatial index has landed; `engine.command-api` fits SolidCraft's plugin idea.
- Verification: extend `verify.zoo` and `verify.agreement`, or work on `verify.regress` (R8); a break-it session any time (prompt in verification.md).

## Owed and waiting on Logan
- R1 exit: a real downloaded STL whose published dimensions match. Clipping decision:
  docs/findings/2026-11-07-clipping-on-real-scans.md (still open).
- cad.solidcraft live replay and R2 exit need SolidCraft running. cad.check's exact version needs SolidCore.
- Publishing (installers, crates.io), Wasm toolchain (`rustup` is not installed), signing and a name check
  (docs/distribution.md).

## Habits that held up
- Claude and Codex share ~/projects/rulercraft: never switch branches there; `git worktree add /tmp/<name> -b <branch>
  origin/main`, remove it after merging.
- Every rule gets a wrong-value run a test fails on, or the rule goes (several were caught today and either tested or removed).
- Merge commits only; wait for CI on the exact head commit before merging; on generated-file conflicts merge main and
  `python3 tools/plan.py render`; take main's HANDOFF.
- Never name other apps in the repo. SolidCraft's plugin design (its issue #1221) is a discussion, not a decision.
- Local only, never committed: fixtures/private (vase scan), local/NOTES.md.
