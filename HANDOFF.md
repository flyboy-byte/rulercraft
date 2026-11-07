# Handoff

Last updated 2026-11-07, Codex. Integration consolidated and audited at Logan's request.

## Shipped and verified

- #18 MCP and #25 strips / #30 MCP checkpoint were merged before this audit.
- #26 corpus, #27 local references, #28 project JSON, #29 3MF and #31 wall thickness are MERGED,
  merge commits only. Logan authorized consolidation in this session after the earlier no-merge rule.
  This is session authorization, not a general change to AGENTS.md/PROCESS.md.
- Checkpoint #32: current docs and findings, hostile 3MF/project bytes, invariance and real-scan run.
  Fixed near-query overflow/underflow, subnormal facing direction, tilted-axis spacing, oversized local
  reference integer traceback. Genuine tilted axes use closest infinite-line distance; numerical
  parallel cutoff 1e-12 preserves analytically parallel fits. Existing known-answer tolerances unchanged.
- Latest full gate: fmt/clippy/workspace tests/plan render+check pass; 105 Rust tests, 2 existing ignored
  scan checks. Both ignored checks separately PASS in release on private data. Python 7 tests, 1 skip
  for standalone CLI-fixture case (cargo supplies the binary and runs it). Local reference capture passes.
- MCP corpus: all 41 fixed dimensions/relations pass. Private scan 412755 vertices/825506 triangles;
  CLI/MCP base plane sigma0.0930818265,5969points, agree within1e-8. Imports0.13-0.35s, MCP fit0.003s.
  Scan plane segmentation0.93s, strip joining2.41s. Raw logs /tmp/checkpoint-*.log; private driver/notes local/.

## Remaining work and cautions

- Read docs/findings/2026-11-07-math-extreme-directions.md and plan/features/math.range.json.
  Public angle helper returns45deg instead of30 for huge finite vectors; normalization of a subnormal
  vector can return NaN/infinity. Caller-facing selection fixed, but src/math/ untouched per lane rule.
- Clipping-cap finding reproduced/re-dated, not silently changed. Sigma3 reaches10rounds on the scan.
- R1 is NOT complete: published-dimension/printed-caliper acceptance and undo/redo remain.
  Engine segmentation, project operations and thickness still lack MCP front ends. No CAD bridge yet.
- User reported13% usage remaining: this run is wrapping the checkpoint, not starting new features.

## Resumption and machine state

Worktree /tmp/rulercraft-io, branch codex/checkpoint-r1-exit. Check #32's live CI/merge state first,
then fetch main, read instructions and run tools/plan.py next immediately before any claim.
README, ROADMAP, implementation plan and generated status reflect the combined state.
Original ~/projects/rulercraft checkout belongs to Claude (paused); it was not switched or edited.
Build cache filled /tmp: only this worktree's target moved to
/home/logan/.cache/rulercraft-codex-target; target is a locally excluded symlink. No source/data removed.

Scratch trap: local codex/mcp-server contains an abandoned unpushed merge of closed/unnecessary #23.
Do not push or use it or apply old MCP stashes. Correct MCP is main / codex/mcp-server-refresh.
Graph #8 and measure #15 were duplicates, closed; use merged APIs. Wrong-value mutations all restored:
latest doubled skew-axis separation (6 vs3), failed as required. No expected values/tolerances loosened.
