# Handoff

Last updated 2026-11-07, Codex. Check `gh pr list` and `python3 tools/plan.py next` before a claim;
main was `88bbaef` when merged into the branch below. Work in a separate worktree, not the shared
`~/projects/rulercraft` checkout.

- PR #51, `mcp.datum`, branch `codex/mcp-datum`, worktree `/tmp/rulercraft-codex-resume`: exposes a
  three-plane datum frame over MCP by reusing the existing plane fits and `align::datum_frame`. The
  tool returns the three fit summaries and the transform; failures leave the mesh unchanged.
- After merging main through #52 into the branch: format, clippy, workspace tests (143 passed, 2
  local-scan tests ignored), and plan check passed. Swapping the primary and secondary inputs made
  the stdio cube check fail; the correct order was restored. Check PR #51's current state and CI.
- Main includes readers, graph, fits, selection, measurements, segmentation, project JSON and MCP
  open/info/select/fit/measure/segment/compare/reverse/history tools. `math.range`, ICP and datum
  frame math are present. The reverse tool provides a CAD recipe and geometric guesses.
- Remaining: replay a CAD recipe against a live SolidCraft server; `cad.check` needs SolidCore
  projection; robust sphere/cylinder fitting, grid patterns, plane-line-point datums and CLI datum
  exposure. R1 still needs Logan's published-dimension exit run and clipping decision
  (`docs/findings/2026-11-07-clipping-on-real-scans.md`).
- Draft PRs claim features; lowest PR number wins. Render generated plan files, and on merge
  conflicts take main's handoff and re-render the generated files. Logan merges; merge commits only.
  Private fixtures and `local/` notes stay local. Do not apply old MCP stashes or use the abandoned
  `codex/mcp-server` branch.
