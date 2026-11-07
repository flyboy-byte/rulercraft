# Handoff

Rewritten (not appended) at the end of every session that leaves work open. Last: 2026-11-07, Claude.

## State (main after #24)

- Readers STL/PLY/OBJ; mesh graph; selection; fits plane/sphere/cylinder; relations; command layer and
  CLI (`rulercraft info`, `rulercraft fit plane|sphere|cylinder`).
- Segmentation: `segment_planes` (flat faces, trimmed so each lies within the limit of its own plane)
  and `segment_cylinders` (holes, bosses, fillets; a triangle's whole surface must lie on the cylinder
  and its normal along the radius). Not on the CLI/MCP yet.
- Verification: invariance, hostile input, a private real scan (local only). Checkpoints are scheduled
  in docs/implementation-plan.md; next is `checkpoint.r1-mcp`, after `mcp.server` merges.
- Open finding: docs/findings/2026-11-07-clipping-on-real-scans.md (undecided).

## In flight (Codex)

- #18 `mcp.server`: was branched before the solver fix (#19); asked to merge main and re-run its stdio
  check. #23 `fit.cylinder` stall claim: probably not needed on main (asked for a mesh if it still fails).

## Next

1. Review and merge #18, then run `checkpoint.r1-mcp` (MCP end to end on the real scan).
2. `segment.strips` (join coarse faceted strips), `corpus.known`, `io.3mf`, `project.file`.
3. Expose segmentation on the CLI/MCP (`rulercraft segment`).

## Habits that held up (2026-11-07)

- Light review, then merge. Re-run `python3 tools/plan.py next` immediately before claiming.
- Each wrong-value run that passed revealed an untested rule; add the check that needs it (or delete the
  rule), never leave it untested.
