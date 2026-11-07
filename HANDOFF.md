# Handoff

Rewritten (not appended) at the end of every session that leaves work open. Last: 2026-11-06, Claude.

## State

- Set up 2026-11-06: docs, plan, plan tool, architecture, Rust workspace, fast CI (about 1 minute).
  Nothing is claimed; no feature work is in progress.
- Done: `io.stl` (ASCII and binary STL, math and wrong-value checks) and `rulercraft info <file.stl>`.

## Starting a session

1. Read AGENTS.md, PROCESS.md, docs/architecture.md, then this file.
2. `gh pr list` and `python3 tools/plan.py next`.
3. Claim one ready feature with a draft PR, read its spec, write the checks, build it, report.

Ready now, independent of each other: `math.core`, `mesh.graph`, `io.ply`, `io.obj`, `io.3mf`,
`corpus.known`, `reference.local`. After `math.core`: `fit.plane`, `fit.sphere`. After `mesh.graph`:
`select.region`, `fit.cylinder` (also needs `math.core`).

## Local only

`local/NOTES.md` (gitignored) on Logan's machine says where local reference tools and the private test
scan are.
