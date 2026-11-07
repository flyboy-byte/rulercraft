# Handoff

Last updated 2026-11-07, Claude. State: main a98c50c + this PR; 106 tests (~1 s), CI green, no open PRs, roadmap 29%.

- Shipped: readers (STL/PLY/OBJ/3MF), mesh graph, selection, plane/sphere/cylinder fits, relations,
  thickness, command layer, CLI (`rulercraft info|fit`), MCP stdio server (`rulercraft-mcp`: open/info,
  select, fit, measure, undo/redo, guide), project file, local-only reference numbers, known-design
  corpus (CLI and MCP), segment_planes / segment_cylinders / join_strips (library only), R1 integration audit.
- R1 is ~82%. Left: Logan's exit run (real downloaded STL, compare hole sizes with the real design);
  segmentation + project tools on the CLI/MCP (`rulercraft segment`); `math.range` (ready); the clipping
  decision (docs/findings/2026-11-07-clipping-on-real-scans.md).
- history.undo (#34) was merged before Codex's wrong-value run and report; status is `implemented`, one run owed.
- join_strips is for tessellated CAD meshes; on scans use segment_cylinders (spec has the limits).
- After R1: R2 `cad.solidcraft` (SolidCraft MCP bridge; the Tangency Labs path), then compare.deviation,
  fit.robust, engine.library. Run `python3 tools/plan.py next` immediately before claiming.
- Habits: claim by draft PR; light review then merge commits; every rule needs a wrong-value run a test
  fails on, or it goes; render generated plan files, never hand-merge them (take main's HANDOFF on conflict).
- Local only: fixtures/private (vase scan, never commit), local/NOTES.md (real-scan numbers, checkpoint log).
