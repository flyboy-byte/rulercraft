# Handoff

Last updated 2026-11-07, Codex. Logan merges; check the current PR list before claiming work.

## Open work

- #69 `fix: respect narrow facing selection angles`, branch `codex/facing-angle`, worktree `/tmp/rulercraft-facing-codex`: a normal about 1e-7 radians from the requested direction was incorrectly selected within 1e-8 radians. The old cosine margin failed the new test; the stable angle calculation passes, including a 90-degree boundary. Next: check hosted CI on the exact head and review/merge when Logan chooses.
- #67 `verify.zoo`, branch `codex/verify-zoo`, worktree `/tmp/rulercraft-zoo-codex`: first zoo slice covers analytic spheres and cylinders across sizes and densities; independent 1% radius mutations failed. More shapes, noise and seeded sweeps remain.
- #66 `fix: bound memory for malformed OBJ corners`, branch `codex/break-input`, worktree `/tmp/rulercraft-break-codex`: 8 MB hostile input peaked at 145452 KiB before and 20624 KiB after. The memory-limited regression failed when the cap was removed. Hosted CI passed on head e48c886.
- #68 `engine.bvh` belongs to the other session; avoid colliding.

## Next

Run `python3 tools/plan.py needs`, choose a useful small change, and run `python3 tools/plan.py next` immediately before claiming. Work in a separate worktree. The clipping decision remains Logan's; private data stays local. Do not apply old MCP stashes or use the abandoned local `codex/mcp-server` branch.
