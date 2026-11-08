# Handoff

Last updated 2026-11-07, Codex. PR #66 is open on `codex/break-input` in `/tmp/rulercraft-break-codex`.

## Open work

- #66 fixes excessive allocation when rejecting OBJ face corners with many slash separators. A generated 8 MB malformed corner peaked at 145452 KiB before and 20624 KiB after; both runs refused it at line 2. A Linux subprocess regression limits address space to 96 MiB; removing the four-field cap made the test fail with an allocation error. Next: check CI on the exact PR head and review/merge when Logan chooses.
- The CLI successfully read the known hole-block STL: 1540 vertices, 3088 triangles, bounds [0,0,0] to [60,30,10] mm. This is a smoke check, not a measurement verification.

## Next

Run `python3 tools/plan.py needs`, pick a useful small change and claim it after `python3 tools/plan.py next`. Work in a separate worktree. Main's earlier handoff gives the wider project state; verify it against current main and open PRs. The clipping decision remains Logan's. Private data stays local. Do not apply old MCP stashes or use the abandoned local `codex/mcp-server` branch.
