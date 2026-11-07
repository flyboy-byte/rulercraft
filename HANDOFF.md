# Handoff

Last updated 2026-11-07, Codex. Continuing small unclaimed work; Logan merges, never Codex.

## Current state

- Main includes MCP #18, strips #25 and checkpoint.r1-mcp #30. Claude is paused; keep its claims clear.
- Corpus #26 (codex/corpus-known): ready. Four closed original design STLs with fixed independent
  answers. Main merged into the branch; regenerated status files. Known corpus ~0.14 s.
- Reference helper #27 (codex/reference-local): ready, previous CI green; updated against main.
  Optional private JSON captures bind exact source bytes by SHA-256, compare CLI numeric fields,
  visibly skip when absent. No independent captures here. Notes local/REFERENCE-CAPTURE.md ignored.
- Project #28 (codex/project-file): ready, previous CI green; updated against main.
  Embedded-source JSON with named fit recipes and cached results. Reopen refits and reports changes.
  CLI project create/fit/info, bounded reads and atomic writes. serde/serde_json MIT OR Apache-2.0,
  D-008. No math changes. New gates after updates recorded in /tmp/*-main-gate.log.
- 3MF #29 (codex/io-3mf): ready, updated against main and full gate passes with MCP.
  Single-model geometry, stored/deflated ZIP, model units, nested transforms, reflected winding,
  byte/geometry/instance limits. Required extensions and cross-part references refuse. zip 2.6 MIT,
  D-009. Original cube and known affine placement checks. /tmp/3mf-main-gate.log.

## Resume

Use /tmp/rulercraft-io (original ~/projects/rulercraft checkout belongs to Claude). Read instructions,
fetch main, inspect live PRs, run python3 tools/plan.py next immediately before claims. Do small
features; user reports 34% usage with 3 hours to reset as of this session. Never poll usage.
MCP checkpoint is DONE #30; R1 exit checkpoint awaits corpus merge. history.undo awaits project.file.
Segmentation is not on CLI/MCP yet. Open clipping finding remains undecided; don't silently change it.
Recheck CI and mergeability; regenerate generated-file conflicts, never hand-merge them.

## Scratch traps and evidence

Local codex/mcp-server contains an abandoned unpushed merge of unnecessary closed #23. Do not push
or use it. Correct code is merged main / codex/mcp-server-refresh / origin/codex/mcp-server. Old MCP
stashes are obsolete. Duplicate graph #8 and measure #15 are closed; use merged main APIs.
Wrong-value runs restored: corpus small-hole radius 1.5 to 1.6; MCP count +1; reference comparison
error +1; project saved cm as mm; 3MF cm factor 10 to 100. Each failed its fixed-answer check.
No tolerances loosened. Logs /tmp/*-wrong-value.log. Corpus is mathematical design evidence, not
printed/caliper or real-scan evidence. Claude's checkpoint scan notes are in original checkout local/.
