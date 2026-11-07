# Handoff

Last updated 2026-11-07, Codex. Pausing at Logan's request for compaction.

## Open work

- MCP #18, remote codex/mcp-server: READY, MERGEABLE; hosted CI passed in 57 s. Local matching
  branch is codex/mcp-server-refresh (44a9181). It exposes mesh.open/info, bounded select pages,
  plane/sphere/cylinder fit, two-selection measurement, schemas and guide over stdio. 78 Rust tests
  and 3 plan tests passed; stdio scenario 0.02 s. rmcp Apache-2.0, Tokio MIT, licences in D-007.
- Corpus #26, codex/corpus-known: READY, MERGEABLE; implementation head f1a3012 hosted CI passed
  in 10 s. Four original closed STLs with fixed JSON answers and tools/generate_known.py source:
  block with 3/5/8 mm holes at 20 mm spacing; stepped cylinder diameters 20/12; radius-12 sphere
  seat; bracket inner faces 24 apart. No dependencies. Corpus check 0.14 s; all stated dimensions,
  closed boundaries and zero non-manifold edges pass. Final follow-up is this handoff only.
- Claude owns segmentation; segment.strips #25 was draft at this handoff. Do not overlap.

## Where to resume

- Worktree /tmp/rulercraft-io, currently codex/corpus-known, linked to ~/projects/rulercraft.
  Original checkout belongs to Claude. Start future sessions in the RulerCraft repo, not scanruler.
- Read AGENTS.md, PROCESS.md, architecture, implementation plan and this handoff. Fetch main and
  check PR states. Run tools/plan.py next immediately before every claim. Continue small unclaimed
  features until usage stops the run; no merges by Codex. MCP checkpoint is due after #18 merges,
  and the R1 exit checkpoint after corpus/segmentation merge. See implementation plan.
- Logan merges with merge commits only. Regenerate plan conflicts; never hand-merge generated files.

## Do not re-raise / scratch traps

- PLY #2, OBJ #5 and cylinder #10 merged. Duplicate graph #8 and measurements #15 are closed;
  use main's graph #7 and measurements #16.
- #23 CLOSED: main's #19 solver fix resolves the symmetric-tube stall. #18 excludes all #23 code.
  Local branch codex/mcp-server contains an abandoned unpushed #23 merge; do NOT push it.
  Use codex/mcp-server-refresh or remote origin/codex/mcp-server instead. Old MCP stashes are
  obsolete recovery snapshots; do not apply them to the finished work.
- Wrong-value checks: MCP triangle count +1 failed (13 vs 12); corpus small-hole radius 1.5 -> 1.6
  failed (diameter 3.2 vs 3). Both restored, answers/tolerances unchanged. Logs /tmp/mcp-server-gate.log,
  /tmp/corpus-gate.log and corresponding *-wrong-value.log files.
- Corpus is design evidence, not printing/caliper or scan evidence. Those exit checks remain.
  Usage helper failed before returning data; no percentage available, do not poll it.
