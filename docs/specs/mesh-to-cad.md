# Mesh to CAD

Status: *Outline*. Features: `cad.solidcraft`, `cad.check`.

From fitted elements, Claude builds the part in SolidCraft over SolidCraft's MCP server (planes as
faces, cylinders as holes or bosses, distances as dimensions). Then the mesh's points are compared
against the rebuilt solid (SolidCore projection) and the deviation reported. SolidCore is an optional
cargo feature; the core crates never depend on it (D-002).

## Recipe (2026-11-07)

`rulercraft_mesh::cad::recipe(planes, cylinders)` turns named fits into plain steps, serialised as JSON:
`base` (the largest plane and its size), `round` (a cylinder within 5° of square to the base, placed by
`u`, `v`, height and diameter in the base's own frame), `parallel_face` (a plane within 5° of parallel,
at a height) and `unplaced` (anything else, with the reason; never guessed). With no plane everything
is `unplaced`. Positions use the base's fitted `basis_u`/`basis_v`, so turning or moving the whole part
leaves every number unchanged (tested). Not done: exposing it on the CLI/MCP, replaying it against a live
SolidCraft server, and the deviation check against the rebuilt solid (`cad.check`).
