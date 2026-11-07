# Handoff

Last updated 2026-11-07, Codex.

- PLY #2, OBJ #5 and cylinder #10 merged. Duplicate graph #8 and measurements #15 closed; use
  main's graph #7 and measurements #16. Logan explicitly reassigned Codex to mcp.server.
- mcp.server #18 on codex/mcp-server, /tmp/rulercraft-io. Implementation and stdio test are saved
  in git stash named "mcp.server implementation and stdio test". Root Cargo.toml/Cargo.lock claimed.
  SDK rmcp 3.9 Apache-2.0, Tokio MIT. Cold compile succeeded after ServerConfig API adjustment.
- MCP stdio test found a cylinder refusal at a genuine minimum: radius 10.000000006470414,
  cost 2.9601323829799404e-13, 96 points, solver stalled after 20 iterations. Separate fix #23,
  codex/cylinder-stall, validates an undamped Gauss-Newton step at the existing 1e-12 solver
  step tolerance. No math files or measurement tolerances changed. Local full gate green.
- Fix regression covers the imported symmetric tube and refusal with one iteration on a tilted
  start. Wrong-value run: diameter 2*r -> 3*r failed exact_tube_and_metadata; restored and green.
- Next: push #23, return to codex/mcp-server, merge fix branch, restore stash, rerun stdio test.
  Then finish MCP wrong-value run/full gate/report, mark #18 ready. Claude owns segment.surfaces.
- Re-run plan.py next with GitHub access immediately before any new claim. Logan merges using
  merge commits only. Generated plan conflicts: render, never hand-merge. Usage helper failed.
