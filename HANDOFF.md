# Handoff

Last updated 2026-11-07, Codex.

- PLY PR #2 merged. OBJ PR #5 hosted gate passed (11 s), no longer listed as open; retargeted
  to main after PLY merged. No new dependencies. Cube/units/fans/refusals and wrong-value checks.
- mesh.graph PR #8, codex/mesh-graph, /tmp/rulercraft-io: normals, adjacency, components,
  boundaries and creases. Last raw result: fmt/clippy/workspace tests/plan render+check pass,
  37 tests about 2 s. Deliberate crease comparison > changed to <: cube expected 12, got 6.
  Restored and green. Known-design corpus block still pending corpus.known.
- Claude claimed select.region in PR #9. Do not overlap; original checkout belongs to Claude.
- Logan requested continued small unclaimed features until usage stops the run. Inspect claims
  and tools/plan.py next, then choose a separate feature PR. No merging: Logan merges, merge
  commits only. Regenerate generated plan files for conflicts; never hand-merge them.
- Usage helper failed with UnboundLocalError before a reading; no usage percentage available.
