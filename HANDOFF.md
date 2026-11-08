# Handoff

Last updated 2026-11-07, Codex. Two PRs are open; Logan merges.

## Open work

- #67 `verify.zoo`, branch `codex/verify-zoo`, worktree `/tmp/rulercraft-zoo-codex`: first shape-zoo slice checks analytic spheres and cylinders at three radii and two point densities. Each 1% radius mutation failed its matching check. This is implemented, not the entire zoo: more shapes, partial surfaces, noise, outliers and seeded sweeps remain. Next: check hosted CI on the PR head and review/merge when Logan chooses.
- #66 `fix: bound memory for malformed OBJ corners`, branch `codex/break-input`, worktree `/tmp/rulercraft-break-codex`: an 8 MB malformed corner peaked at 145452 KiB before and 20624 KiB after. A 96 MiB subprocess regression failed when the cap was removed. Hosted CI passed on head e48c886. Next: review/merge when Logan chooses.

## Next

Run `python3 tools/plan.py needs`, pick a useful small change and claim it after `python3 tools/plan.py next`. Use a separate worktree. The clipping decision remains Logan's. Private data stays local. Do not apply old MCP stashes or use the abandoned local `codex/mcp-server` branch.
