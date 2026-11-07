# Handoff

Last updated 2026-11-07, Claude. State: main after #25 (segment.strips) and #18 (mcp.server).

- Shipped today: STL/PLY/OBJ readers, mesh graph, selection, plane/sphere/cylinder fits, relations,
  command layer, CLI (`rulercraft info|fit`), MCP stdio server (`rulercraft-mcp`: mesh.open/info, select,
  fit, measure, guide), segment_planes, segment_cylinders, join_strips. 87 tests, about 1 s.
- checkpoint.r1-mcp done: the MCP server driven end to end on the private vase scan matches the CLI to 15
  digits and survives bad input (numbers in local/NOTES.md). Strips invariance (turn, move, rescale
  0.1x-25.4x) and empty/degenerate meshes added to tests/segment_strips.rs.
- join_strips is meant for tessellated CAD meshes. On a scan it takes 1.5 s but reports about 2,000 small
  low-coverage patches; use segment_cylinders there. Noise near half the 0.2 mm limit splits a surface
  (known limit, docs/specs/segmentation.md).
- Not on the CLI/MCP yet: segmentation (`rulercraft segment`). Open decision: docs/findings/2026-11-07-clipping-on-real-scans.md.
- Next ready features (run `python3 tools/plan.py next` immediately before claiming): corpus.known,
  io.3mf, project.file, reference.local, history.undo. Then checkpoint.r1-exit and Logan's R1 exit run.
- Habits: light review then merge commits; claim by draft PR; every rule needs a wrong-value run a test fails
  on, or it goes; render generated plan files, never hand-merge them.
