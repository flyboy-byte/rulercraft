# Handoff

Last updated 2026-11-07, Codex. Logan merges; check the current PR list before claiming work.

## Open work

- #75 `test: check robust sphere fits across scales and outliers`, branch `codex/zoo-outliers`, worktree `/tmp/rulercraft-zoo-outliers`: analytic spheres at three radii, three outlier rates and two seeds; 0.01% bounded radial noise and a 0.2% radius/centre allowance. A 1% radius mutation made the robust fit fail. Next: check hosted CI on the exact PR head and review/merge when Logan chooses.
- #72 `test: extend shape zoo with partial noisy surfaces`, branch `codex/zoo-noisy`: 60-degree sphere caps and 120-degree cylinder arcs across three scales and three seeds. Independent 2% radius mutations failed. Hosted CI passed on head 56e7096.
- #71 `fix: keep boundary hits for parallel rays`, branch `codex/bvh-negative-zero`: a known triangle hit was lost with negative zero and with a subnormal ray component. New tests failed before the fix and on an exclusive-boundary mutation. Hosted CI passed on head a7ef880.
- #73 `engine.parallel` is another session's claim; avoid colliding.

## Next

Run `python3 tools/plan.py needs`, choose a useful small change, and run `python3 tools/plan.py next` immediately before claiming. Work in a separate worktree. The clipping decision remains Logan's; private data stays local. Do not apply old MCP stashes or use the abandoned local `codex/mcp-server` branch.
