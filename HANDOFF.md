# Handoff

Last updated 2026-11-07, Codex. Logan merges; check the current PR list before claiming work.

## Open work

- #71 `fix: keep boundary hits for parallel rays`, branch `codex/bvh-negative-zero`, worktree `/tmp/rulercraft-bvh-zero`: the tree missed a known triangle hit at t=1 when a parallel component was negative zero, and also for a tiny nonzero component with an infinite reciprocal. The new test failed before the fix. Rejecting the inclusive upper boundary on purpose also made it fail. Next: check CI on the exact PR head and review/merge when Logan chooses.
- #67 `verify.zoo`, branch `codex/verify-zoo`: first zoo slice covers analytic spheres and cylinders across sizes and densities. More shapes, noise and seeded sweeps remain. The remote branch was updated separately; check its current head before any edit.
- #69 `fix: respect narrow facing selection angles`, branch `codex/facing-angle`: an approximately 1e-7-radian normal was wrongly included in a 1e-8-radian selection. A known-angle regression failed before the fix and under a deliberate wrong-value change.
- #66 OBJ corner memory fix and #68 spatial index have merged. The latter is the base of #71.

## Next

Run `python3 tools/plan.py needs`, choose a useful small change, and run `python3 tools/plan.py next` immediately before claiming. Work in a separate worktree. The clipping decision remains Logan's; private data stays local. Do not apply old MCP stashes or use the abandoned local `codex/mcp-server` branch.
