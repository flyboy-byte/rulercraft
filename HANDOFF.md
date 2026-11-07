# Handoff

Last updated 2026-11-07, Codex. Main was `88bbaef` at the start of the work below. Check open PRs
and run `python3 tools/plan.py next` immediately before claiming anything; use a separate worktree.

- PR #51 `mcp.datum`, branch `codex/mcp-datum`, worktree `/tmp/rulercraft-codex-resume`: MCP tool
  fits three plane selections and returns their datum-frame transform. Its stdio check recovers the
  known cube frame and checks refusals; swapping two datum inputs made that check fail. Local gates
  passed (143 tests, 2 local-scan tests ignored), and hosted CI was green when last checked.
- PR #53 `mcp.thickness`, branch `codex/mcp-thickness`, worktree `/tmp/rulercraft-codex-thickness`:
  MCP tool returns paged directional thickness samples using the existing engine work cap. The stdio
  check measures a 6 mm wall, pages results and verifies missing hits and refusals. Doubling the
  reported distance made it fail; the correct code was restored. Local gates passed (144 tests,
  2 local-scan tests ignored). Check the current PR and hosted CI state.
- Main includes importers, selection, fits, measurements, segmentation, projects, ICP, reverse
  geometry and MCP open/info/select/fit/measure/segment/compare/reverse/history tools. Both open PRs
  touch MCP and may need main merged into their branches after the other lands. Re-render generated
  plan files on conflicts; take main's HANDOFF and rewrite it for current state.
- Remaining: replay a CAD recipe against a live SolidCraft server; `cad.check` needs SolidCore
  projection; robust sphere/cylinder fitting, grid patterns and plane-line-point datums. R1 still
  needs Logan's published-dimension exit run and clipping decision
  (`docs/findings/2026-11-07-clipping-on-real-scans.md`).
- Claims are draft PRs; lowest number wins. Logan merges, using merge commits. Private fixtures and
  `local/` notes stay local. Do not apply old MCP stashes or use the abandoned `codex/mcp-server`.
