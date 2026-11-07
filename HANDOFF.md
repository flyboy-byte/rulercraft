# Handoff

Last updated 2026-11-07, Codex. Consolidation and integration audit are finished.

## State

- #26 corpus, #27 local references, #28 projects, #29 3MF, #31 thickness and #32 integration audit
  are MERGED with merge commits. Main's integration audit has 105 passing Rust tests and both private
  scan checks pass in release. All41 fixed corpus dimensions/relations pass through MCP. Docs updated.
- Audit fixes: near overflow/underflow, subnormal facing direction, tilted-axis distance, oversized
  reference integer errors. Math-range and clipping-cap findings remain open; src/math/ untouched.
- Final small follow-up #33, codex/cli-import-limits, is READY for review; not merged.
  CLI now checks declared file size before allocation and reuses bounded reads. Sparse2GiB+1 file
  tested under512MiB child address-space cap gives byte-limit error instead of out-of-memory.
  Deliberate off-by-one metadata guard failed that regression and was restored. Full local gate passes.
  No dependencies, CI/workflow changes or math edits. Check hosted CI before merging.
- User reported13% remaining and asked whether work was done. Do not infer a request for further
  feature work from this handoff. The persistent continuing goal is still active and may trigger
  automatic continuations; user must pause it to prevent those.

## Remaining

math.range finding: huge-vector angle45 instead of30 and subnormal normalization NaN/infinity.
Clipping still reaches10 rounds on the real scan. Undo/redo and segmentation/project/thickness MCP
front ends remain unfinished. R1 physical/published-dimension acceptance is not complete.

## Resumption / machine

Worktree /tmp/rulercraft-io, branch codex/cli-import-limits. Original checkout belongs to paused Claude.
Fetch main, inspect live PRs and run plan.py next immediately before any future authorized claim.
Build cache moved off full /tmp to /home/logan/.cache/rulercraft-codex-target; target is an excluded
symlink. Private scan/reference/driver/notes stay ignored. Raw logs /tmp/checkpoint-*.log and
/tmp/cli-limits-*.log. Local gate: fmt, clippy, workspace tests, Python tests, plan render/check pass.

Do not use local codex/mcp-server: abandoned unpushed merge of closed/unnecessary #23.
Correct MCP is merged main / codex/mcp-server-refresh. Do not apply old MCP stashes.
Duplicates graph #8 and measure #15 closed. No expected answers or tolerances loosened.
