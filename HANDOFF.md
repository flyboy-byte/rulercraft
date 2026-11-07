# Handoff

Last updated 2026-11-07, Codex.

- PLY #2 and OBJ #5 merged. Cube encodings, units, fan triangulation, negative OBJ indices,
  refusal/limits and CLI checks passed; each deliberate wrong-value check failed as intended.
- mesh.graph duplicate #8 closed: lower-numbered Claude PR #7 won and merged. Do not merge #8.
  select.region #9 also merged. Use main's graph API, not the abandoned #8 branch.
- fit.cylinder #10, codex/fit-cylinder, worktree /tmp/rulercraft-io: ready for review after report.
  Last raw result: fmt/clippy/workspace tests/plan render+check pass, 50 tests, about 2 s.
  Known radius-10 tube, 120-degree arc, tilted axis, inaccurate-normal refinement, clipping and
  flat/invalid/limit refusals. Wrong-value: diameter multiplier 2 -> 3 failed exact_tube_and_metadata;
  restored and green. No math files touched and no new dependencies.
- Spec records deterministic starts, work limits and clipping stop at f32 coordinate quantization.
  Designed corpus and large real meshes not yet checked; starts are not globally convergent.
- Logan requested continued small unclaimed features until usage stops the run. Next likely
  measure.relations, stacked on #10 if not merged. Recheck claims first. Logan merges with merge
  commits. Regenerate generated plan files on conflict; never hand-merge them.
- Original checkout belongs to Claude; leave it alone. Usage helper failed before returning data.
