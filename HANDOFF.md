# Handoff

Last updated 2026-11-07, Codex.

- PLY #2, OBJ #5 and fit.cylinder #10 merged. No new dependencies in those features and no math
  files changed. Each has known-answer geometry and a deliberate wrong-value check recorded.
- Duplicate mesh.graph #8 was closed under lowest-PR precedence; use Claude's merged #7 API.
- measure.relations #15, codex/measure-relations, /tmp/rulercraft-io: typed point/centre, plane
  and axis measurements plus `rulercraft measure first.json second.json` (saved fit JSON or
  descriptors). Retargeted to main after #10 merged; merged main, regenerated plan files.
  Last raw result: fmt/clippy/workspace tests/plan render+check pass, 59 Rust tests, 3 plan tests,
  about 3 s. Wrong value: reversed point-plane subtraction; expected -5, got +5. Restored/green.
- Spec records acute angles, first-plane centre convention, parallel-axis threshold, signed point
  distance, mm/radian engine and degree CLI output. No new dependencies. Known-design corpus
  still pending; cylinder descriptors accepted but cylinder CLI fit integration is not in #15.
- Logan requested continued small unclaimed features until usage stops the run. Before choosing,
  fetch main and run tools/plan.py next with GitHub access; inspect docs/implementation-plan.md.
  Original checkout belongs to Claude; leave it alone. Logan merges with merge commits only.
  Regenerate generated files on conflicts. Usage helper failed before returning a reading.
