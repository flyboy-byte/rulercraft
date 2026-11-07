# Handoff

Last updated 2026-11-07, Codex.

- PLY #2, OBJ #5 and cylinder #10 merged. Duplicate graph #8 and measurements #15 were closed;
  use main's graph #7 and measurements #16. Logan assigned Codex to mcp.server next.
- MCP #18: /tmp/rulercraft-io, local branch codex/mcp-server-refresh tracks remote
  codex/mcp-server. Ready after pushing implementation/report. Merged current main (through #24),
  including solver fix #19; raw stdio test now passes without any #23 code.
- Closed #23 at Logan's direction: #19 resolves the symmetric tube stall. The MCP branch contains
  no cylinder or math changes. The abandoned prerequisite stays on its own unmerged branch.
- New rulercraft-mcp binary: mesh.open/info, select with capped pages, plane/sphere/cylinder fit,
  measure by fitting two selections, guide and typed JSON schemas. One mesh per process, failed
  opens preserve state. rmcp 3.9 Apache-2.0; Tokio MIT; serde/serde_json MIT OR Apache-2.0.
- Last raw result: fmt/clippy/workspace tests/plan render+check and 3 plan tests pass. 78 Rust tests
  pass, one existing ignored; stdio scenario 0.02 s. Wrong-value run: triangle summary +1 caused
  stdio cube check to fail (13 vs 12); restored and green. Full log /tmp/mcp-server-gate.log.
- Real-client/real-scan MCP runs remain for the scheduled checkpoint. No saving/history or network
  transport yet. Claude owns segmentation. Original checkout belongs to Claude; leave it alone.
- Logan requested continuing small unclaimed features until usage stops the run. Re-run
  tools/plan.py next with GitHub access immediately before claiming. Logan merges using merge
  commits; render generated plan conflicts, never hand-merge. Usage helper failed before data.
