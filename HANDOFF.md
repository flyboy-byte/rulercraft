# Handoff

Rewritten (not appended) at the end of every session that leaves work open. Last: 2026-11-06, Claude.

## State

- Repository set up 2026-11-06: docs, plan, plan tool, Rust workspace, fast CI.
  Setup only: no feature work is claimed or started beyond the STL reader.
- `rulercraft-mesh` reads ASCII and binary STL (`io.stl`: math and wrong-value checks done;
  known-design check waits on `corpus.known`).
- `rulercraft info <file.stl>` prints the summary.

## Next (when Logan starts work)

1. `mesh.graph`, then `select.region` and `fit.plane`.
2. `corpus.known`: our own designed test parts with known dimensions (committed), plus downloaded models
   with published dimensions kept in `fixtures/private/`.
3. `reference.local`: optional local reference numbers (see `local/` on Logan's machine).
