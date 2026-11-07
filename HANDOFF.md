# Handoff

Rewritten (not appended) at the end of every session that leaves work open. Last: 2026-11-07, Claude.

## State (main after #16)

- Readers: STL, PLY, OBJ (`read_mesh` by extension). Mesh graph: normals, edges, components,
  boundaries, creases. Selection: box, near, facing, explicit, thinned.
- Fits: plane, sphere (optional held diameter), cylinder (Codex). Relations between fitted elements:
  plane/plane, centre/plane, centre/centre, axis/plane, axis/axis (hole spacing), centre/axis.
- Command layer `rulercraft_mesh::commands` shared by front ends; CLI `rulercraft info` and
  `rulercraft fit <plane|sphere|cylinder>` with box/near/facing selections, JSON out.
- Process notes from 2026-11-07: light review then merge; stale generated files are a note, not a
  failure; `plan.py next` shows claims from open PR titles (two sessions once both built mesh.graph).

## Next (docs/implementation-plan.md has the full order)

1. `mcp.server`: the R1 critical path. New crate over `commands`, plus a measure command.
2. `segment.surfaces` (split it first), `corpus.known` (generated parts with exact answers), `io.3mf`,
   `project.file`.
3. STL overflow fix (small): refuse coordinates that overflow f32 after unit conversion.

## Local only

`local/NOTES.md` (gitignored) on Logan's machine says where local reference tools and the private test
scan are.
