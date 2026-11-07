# Handoff

Last updated 2026-11-07, Codex. Main was `88bbaef` at the start of the work below. Check open PRs
and run `python3 tools/plan.py next` immediately before a claim; use a separate worktree.

- PR #51 `mcp.datum`, branch `codex/mcp-datum`, worktree `/tmp/rulercraft-codex-resume`: MCP tool
  fits three plane selections and returns their datum frame. Known cube frame and refusal checks pass;
  swapping two datum inputs failed the check. Local gates passed (143 tests; 2 local tests ignored),
  and hosted CI was green when last checked.
- PR #53 `mcp.thickness`, branch `codex/mcp-thickness`, worktree `/tmp/rulercraft-codex-thickness`:
  MCP tool returns bounded paged directional thickness samples. A known 6 mm wall, paging and error
  checks pass; doubling the reported distance failed the check. Local gates passed (144 tests; 2
  local tests ignored), and hosted CI was green when last checked.
- PR #54 `mcp.fit-validation`, branch `codex/mcp-fit-validation`, worktree
  `/tmp/rulercraft-codex-cli-thickness`: fixes a confirmed inconsistency: robust plane fits silently
  accepted `known_diameter` while ordinary plane fits rejected it. Both now refuse it. The stdio test
  failed with the old validation restored. Local gates passed (143 tests; 2 local tests ignored).
- All three PRs touch MCP and may need main merged into their branches after another lands. Resolve
  generated plan files with `python3 tools/plan.py render`; on conflict take main's HANDOFF and rewrite
  it for the current state. Check current PR and hosted CI state before acting.
- Remaining: live CAD recipe replay, `cad.check` SolidCore projection, robust sphere/cylinder fits,
  grid patterns, plane-line-point datums, Logan's R1 published-dimension exit run and clipping
  decision (`docs/findings/2026-11-07-clipping-on-real-scans.md`).
- Lowest numbered draft PR wins a claim. Logan merges with merge commits. Private fixtures and `local/`
  notes stay local. Do not apply old MCP stashes or use the abandoned `codex/mcp-server` branch.
