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

## Check (2026-11-07)

`cad::check(mesh, planes, cylinders, tolerance)` is an in-house coverage check on the recipe's features. Each
vertex is matched to the nearest feature it lies on; a plane covers vertices within one span of its centre
across the plane, a cylinder covers vertices within its measured length (plus display extensions) along its
axis. It reports vertices, the number and fraction explained, RMS and largest distance of the explained
ones, the unexplained vertex ids, and explained counts per feature. A vertex further than `tolerance` from
every feature is unexplained. This tells a person how much of the part the recipe accounts for, and where
it does not (bosses, fillets or chamfers it has no step for appear as unexplained patches).

It is not the check the roadmap's R2 exit asks for. That compares the mesh with the solid a CAD program
built from the recipe (SolidCore projection, optional cargo feature, D-002) and needs SolidCraft running.
The MCP `reverse` tool includes this check (tolerance 0.1 mm) as `recipe_check`.
