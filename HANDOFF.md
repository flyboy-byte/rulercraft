# Handoff

Last updated 2026-11-07, Codex.

- PLY PR #2 ready, branch codex/io-ply. Hosted gate passed (13 s). ASCII/LE/BE cube exact;
  dispatcher and CLI; no dependencies. Wrong-value fan check failed then restored.
- OBJ PR #5 on codex/io-obj, stacked on #2. Local fmt/clippy/workspace tests/plan check pass
  (29 tests, about 2 s). Negative indices, slash corners, quads/n-gons, units, bounds and refusals.
  Wrong-value fan k+1 -> k caused cube test to fail; restored and all green.
- Worktree: /tmp/rulercraft-io. Original checkout belongs to Claude's sphere work; leave it alone.
- Logan requested continued small unclaimed features until usage stops the run. Next: inspect ready
  list and claims, likely mesh.graph. No merging: Logan merges, merge commits only. After #2 merges,
  retarget #5 to main; regenerate generated plan files for conflicts, never hand-merge them.
- Known-design corpus block remains pending corpus.known. No real-world corpus or reference used.
- Usage helper failed with UnboundLocalError before a reading; no usage percentage available.
