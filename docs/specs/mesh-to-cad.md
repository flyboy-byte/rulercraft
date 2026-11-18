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

- **Base and frame.** Flat and round regions are found as by `segment`; the largest flat region is the base. The
  opposite face is the largest other face parallel to it (within 5°); the frame's normal points from the base into
  the part, so the base is at height 0 and the opposite face at the thickness. The in-plane axes follow the base
  outline's edges: the rotation of the smallest rectangle around the base face's points (rotating calipers over
  their convex hull). The frame is right-handed so the part is never mirrored. With no parallel face the scan is
  refused: the thickness is unknown.
- **Plate.** A box over that rectangle, from the base to the opposite face; the box's corner is the origin.
- **Other parallel faces.** An upward face between base and top is a **pocket or step floor**: a box cut from it
  through the top over its own rectangle. An upward face above the top is a **pad**: a box joined from the top. A
  face whose edges are turned to the plate's by more than 1° is reported, as is a downward face inside the plate
  (an undercut) or any other face outside the plate.
- **Round regions** square to the base (within 5°): one whose surface faces its axis (vertex normals point
  inward) is a **hole**, cut at its fitted position and diameter; it is through when its wall reaches a downward
  face (the base or one below) and an upward face (the top, a pocket floor, a pad), blind when it reaches one.
  One whose surface faces away and stands on an upward face is a **boss**, joined from just inside that face up to
  its top. A boss's top and a blind hole's floor count as explained. A round wall that runs the full thickness
  facing outward is part of the outline (below), reported as a round outline only when the outline stays a rectangle. Tolerances: heights within 2 % of the thickness (at least 0.05 mm).
- **Outline.** The base face's outer boundary is traced and the plate is that outline extruded from a sketch, not
  its enclosing rectangle, when the outline covers less than 98 % of the rectangle. A boundary that fits one circle
  within the tolerance is a circle; any other is a polygon, its corners the points a straight-edge approximation
  needs within the tolerance (at most 200). A plate that fills its rectangle stays a box. Holes and bosses must lie
  inside the traced outline. Curves in a polygon outline are runs of short straight edges, reported as
  `outline_faceted`; cutouts in the base that are not round holes are reported as `cutout_not_round`.
- **Cutouts.** An inner loop of the base face that is not a circle (a circle needs 12 or more boundary points on
  one circle within the tolerance, since four corners always do) and that the top face repeats at the same place
  and size is a window cut through the plate: a sketch of its corners, cut full thickness (`details.cutouts`).
  Counterbores need nothing special: a wide blind round and a narrow through round on one axis are two cuts.
- **Parameters.** `length`, `width`, `thickness`, `hole<i>_diameter`, `boss<i>_diameter`, `boss<i>_height`,
  `pocket<i>_depth`, `pad<i>_height` are named parameters (to a micrometre). Pockets hang from `thickness`, so
  changing the thickness keeps their depth.
- **Loss report.** Everything not rebuilt is an item, never a guess: `slanted_face`, `tilted_round`,
  `round_outside`, `hole_internal`, `boss` (not standing on an upward face), `round_outline`, `pocket_rotated`,
  `pad_rotated`, `undercut`, `below_base`, and simplifications `outline_rectangle`, `outline_faceted`, `cutout_not_round`, `pocket_rectangle`,
  `pad_rectangle` (a face covering less than 98 % of its rectangle). `details` carries the plate, holes, bosses,
  pockets and pads with their measured sizes and fit quality, and the frame that maps the design back onto the scan.

Known answers: an L-shaped plate (fixtures/known/l-plate.stl, 60 x 20 plus 20 x 20, 8 mm thick, Ø6 hole) rebuilds
as a six-corner sketch with SolidCraft's own volume, 12,573.805 mm³; a Ø50 x 6 round plate with a Ø8 hole as a circle
sketch, 11,479.380 mm³. The hole block rebuilds as 60 x 30 x 10 with through holes of 3, 5 and 8 mm that map back to
x = 10, 30, 50, y = 15, also after the scan is turned and moved. The bracket (fixtures/known/bracket.stl, made by
SolidCraft from bracket.solidcraft.json) rebuilds as 80 x 50 x 8 with its Ø10 x 6 boss, 20 x 15 x 3 pocket, Ø6
through hole and Ø4 blind hole each mapped back to where they were designed, also when turned 30° in its plane and
tilted and moved; SolidCraft measures the rebuilt bracket at 31,282.21237390821 mm³, its original 31,282.212373908205.
The stepped cylinder rebuilds its upper step as a boss and reports its round outline; a lone sheet is refused.

Checked in SolidCraft itself (its CLI built from tangency-labs/solidcraft at 4711df6a, 2026-11-18, `solidcraft-cli eval` of
the written design): the hole block's design rebuilds with volume 17,230.30980 mm³ (60 x 30 x 10 less three
10 mm cylinders of 3, 5 and 8 mm: 17,230.30980 by hand), surface 5,748.717 mm² (5,748.717 by hand) and 9 faces;
the turned and moved scan gives the same three numbers. The JSON shape (parameters with `unit` and `model`,
numeric corners and bases) was copied from a design SolidCraft saved from the equivalent commands.
