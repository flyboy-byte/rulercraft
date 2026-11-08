# Handoff

Last updated 2026-11-07, Codex. Logan merges; check the current PR list before claiming work.

## Open work

- #72 `test: extend shape zoo with partial noisy surfaces`, branch `codex/zoo-noisy`, worktree `/tmp/rulercraft-zoo-noisy`: 60-degree sphere caps and 120-degree cylinder arcs at three scales and three deterministic noise seeds, with noise at most 0.1% and a 1% radius/centre allowance. Independent 2% sphere and cylinder radius mutations each failed. Next: check hosted CI and review/merge when Logan chooses.
- #71 `fix: keep boundary hits for parallel rays`, branch `codex/bvh-negative-zero`: a known triangle hit was lost with negative zero and with a subnormal ray component. The new test failed before the fix and when the inclusive upper boundary was deliberately made exclusive.
- #66 (OBJ corner memory), #67 (first shape zoo), #68 (spatial index) and #69 (narrow facing angles) have merged.

## Next

Run `python3 tools/plan.py needs`, choose a useful small change, and run `python3 tools/plan.py next` immediately before claiming. Work in a separate worktree. The clipping decision remains Logan's; private data stays local. Do not apply old MCP stashes or use the abandoned local `codex/mcp-server` branch.
