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

## SolidCraft design (2026-11-18)

`rulercraft_mesh::solidcraft::design(mesh, name)` turns a scan straight into an editable SolidCraft part, written
as the plain JSON design SolidCraft opens (format `solidcraft/57`; later SolidCraft versions upgrade it on open).
It is the core of the SolidCraft importer plugin (SolidCraft's book, "Writing a plugin"), and the CLI's
`rulercraft solidcraft <mesh> [--units U] [--out design.json]` gives the same result without the plugin.

- **Base.** Flat and round regions are found as by `segment`; the largest flat region is the base. Its frame
  (u, v, normal) is made right-handed so the part is never mirrored.
- **Plate.** A box over the base outline (the base face's own points, min to max along u and v), as thick as
  the parallel face (within 5°) farthest from the base. The box's corner is the design's origin. With no
  parallel face the scan is refused: the thickness is unknown.
- **Holes.** A round region square to the base (within 5°) whose surface faces its axis (its vertex normals
  point inward) is a hole, cut as a cylinder at its fitted position and diameter. A hole whose wall reaches both
  faces (within 2 % of the thickness, at least 0.05 mm) is cut through, past both faces; one that reaches one
  face is cut blind from that face to the wall's end.
- **Parameters.** `length`, `width`, `thickness` and `hole<i>_diameter` are named parameters (to a
  micrometre), and the features use them, so the part is edited by changing a number.
- **Loss report.** Everything not rebuilt is an item, never a guess: slanted faces (`slanted_face`), round
  features not square to the base (`tilted_round`), bosses and pins (`boss`, surface facing away from the
  axis), holes outside the outline (`hole_outside`) or opening on neither face (`hole_internal`), and an
  outline that is not a full rectangle (`outline_rectangle`, when the base face covers less than 98 % of what
  its rectangle leaves after through holes). `details` carries the measured sizes, each hole's sigma and
  coverage, and the frame that maps the design back onto the scan.

Known answers: the hole block rebuilds as 60 x 30 x 10 with through holes of 3, 5 and 8 mm that map back to
x = 10, 30, 50, y = 15, also after the scan is turned and moved; the stepped cylinder reports two bosses and a
simplified outline; a lone sheet is refused.
