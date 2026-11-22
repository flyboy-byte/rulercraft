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
  outline's edges. They come from the walls square to the base (within 5°): the area-weighted mean of their facings
  taken four-fold, so walls at right angles agree, counting walls of at least a tenth of the largest wall's area
  (round holes' facets are small walls at every angle) and only those within 1° of the peak. When fewer than half
  the counted wall area agrees (a hexagon), or there are no walls, the rotation of the smallest rectangle around the
  base face's points (rotating calipers over their convex hull) decides; it also picks which quarter turn is used.
  On a scan the base face's ragged boundary can turn the smallest rectangle by a fraction of a degree; the walls do
  not. The frame is right-handed so the part is never mirrored. With no parallel face the scan is
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
  its enclosing rectangle, when the outline covers less than 98 % of the rectangle. On a mesh with median adjacent
  face angle at most 2°, also trace the outline if a boundary point is more than the outline tolerance from all
  four sides of the rectangle. This retains small notches with a clear inset boundary. A boundary that fits one circle
  within the tolerance is a circle; any other is a polygon, its corners the points a straight-edge approximation
  needs within the tolerance (at most 200). A plate that fills its rectangle stays a box. Holes and bosses must lie
  inside the traced outline. Curves in a polygon outline are runs of short straight edges, reported as
  `outline_faceted`; cutouts in the base that are not round holes are reported as `cutout_not_round`.
- **Cutouts.** An inner loop of the base face that is not a circle (a circle needs 12 or more boundary points on
  one circle within the tolerance, since four corners always do) and that the top face repeats at the same place
  and size is a window cut through the plate: a sketch of its corners, cut full thickness (`details.cutouts`).
  Counterbores need nothing special: a wide blind round and a narrow through round on one axis are two cuts.
- **Countersinks.** Triangles slanted to the base (between 5° and 85°) are grouped by shared edges. A group whose
  normals, followed across the base, meet at one axis, whose radius grows in a straight line with height within the
  tolerance, and that goes round (32 of 36 ten-degree sectors) is a cone square to the base. A cone that widens from
  a hole's top on its axis to the top face is that hole's countersink: the hole opens on the top, and a cone cut
  (`hole<i>_csk_diameter`, `hole<i>_csk_depth`, following `thickness`) rebuilds it. Strips of the cone that the
  cylinder search fits as tilted rounds are not reported.
- **Chamfers.** A group that is no cone is split by facing (1°). A face that faces up, reaches the top face and goes
  down a wall of a straight outline edge (its lowest points on the wall, its sides within the edge's ends) is a bevel
  on that edge, `width` across the top and `height` down the wall. Bevels of one size share one chamfer feature
  (`chamfer<i>_size`, or `chamfer<i>_width` and `chamfer<i>_height`), its edges named by their midpoints at the top.
  Bevels under the plate and on round outlines stay in the report as slanted faces.
- **Fillets.** On each straight top edge of the outline, the slanted triangles facing straight out over it (1°) and
  lying within the thickness of it are cut across: when their cross-section is one circle (within the tolerance)
  touching both the top face and the wall, the edge is rounded, rebuilt by a fillet feature (`fillet<i>_radius`,
  one per radius). Slanted triangles inside a filleted edge's round band past its ends are the corner patches where
  two fillets meet, and are part of them.
- **Flat face heights.** Region growing takes triangles up to 10° off a face, which can include a fillet's first
  facets; a flat face's height is therefore the area-weighted centre of its triangles within 1° of it.
- **Scans.** A scan's mesh is dense and even, its edges softened by about a cell, its triangles tilted by noise;
  a CAD mesh's faces are exactly flat. The rebuild measures which it has and adapts, leaving a CAD mesh's result
  unchanged:
  - *Roughness* is the median angle between neighbouring triangles (0° on a CAD mesh). Flat regions grow across
    2.5 times it (10° to 30°) and round regions across 2.5 times it (15° to 30°); their distance limit (0.2 mm)
    still stops them at real edges. Above 2° a face must cover 93 % of its rectangle, not 98 %, to count as whole,
    and pockets and pads may turn 2.5°, not 1°, before they count as turned.
  - *Reach*, how far a softened edge pulls a face's boundary or a round's end from the real edge, is three times
    the mesh's median edge length, at most 1 mm, at least the tolerance. The plate's, pockets' and pads' sides move
    onto the walls that bound them (each wall within 1° of the side's direction, within the reach, beside it in
    height and along it), since a scan's faces stop short of their edges; rounds' ends move to the nearest face
    within the reach, so holes and bosses keep their real depths and heights.
  - Round regions start only from triangles outside flat regions of 200 triangles or more, and no round is wider
    than half the mesh's diagonal: on a noisy face every triangle is curving, and near-flat "cylinders" grown across
    it cost most of the time.
  - Not features: rounds square to the base covering under 200° of a circle (softened vertical edges, reported as
    `rounded_edges`); rounds thinner than half the reach or coaxial with a round already taken; rounds lying along
    an edge whose radius is within the reach, whose arc is under twice it, or covering under 45°; and slanted
    faces, outside rounds and slivers on the base's or top's rim with under 1 % of the base's area. Those are
    counted in one `small_faces` item.
  - Outlines and windows: a traced boundary is simplified within the reach (not the tolerance) on a rough mesh;
    each edge then moves onto the wall that bounds it (normal within 10° of the side away from the material, within
    the reach, overlapping along the edge), edges on one wall merge, and a short edge no wall bounds between two that
    walls do is a softened corner whose neighbours meet instead. A round outline takes the circle of the outer wall's
    own round region when one lies on it. Boundary chains that touch at a vertex (common on a noisy face) are walked
    through it. A window needs at least 1 % of the base's area.
  - Countersinks: when a hole's wall stops short of the top and no cone was found from facets, the triangles around
    its axis between the two that face up and in at 20° to 75° are fit with radius a straight line in height (points
    off it dropped, fit again); widening upward at a half-angle of 20° to 75°, all the way round, meeting the hole's
    wall within twice the reach of its end, it is the countersink.
  - Bevels take their sizes from where their own fitted plane meets the top and the wall. Fillets: on each straight
    top edge, the triangles beside it facing 15° to 75° from the base's normal and within 20° of the edge's outward
    side; as a fillet touches the top and the wall, each point gives a radius, and the median is the fillet's. It is
    one when 60 % of the points lie within twice the tolerance of that circle (RMS within the tolerance), across 30°
    of arc, larger than the reach, and its facets do not mostly (half or more within 6° of their middle) face one way,
    which would be a bevel. A slanted face, a bevel's face or a tilted round counts as explained when three quarters
    (a round: half) of it is already rebuilt or on a large flat face.
  Known answer (crates/rulercraft-mesh/tests/scan_like.rs): a 60 x 40 x 8 plate with a Ø6 through hole, a Ø4 hole
  5 deep, a Ø10 boss 5 tall and a 16 x 10 pocket 3 deep, meshed by surface nets at 0.5 mm, turned 23° about a
  tilted axis, 0.02 mm noise on every vertex: every size within 0.05 mm (the pocket's within 0.1 mm), nothing
  dropped, the scan within 0.1 mm of the rebuilt part (95th percentile). And a 50 x 30 x 8 plate with a 2 x 1 bevel,
  an R2 round, a Ø5 hole countersunk 90° to Ø10 and a 10 x 8 window, meshed at 0.4 mm, turned and with noise: each
  found within 0.1 mm. Synthetic scans of the SolidCraft-made fixtures (marching cubes at 0.35 mm, 0.02 mm noise,
  turned 17°) rebuild within 0.25 % of SolidCraft's own volume for each.
- **Check.** The rebuilt part is also a signed-distance field (plate outline, pads and bosses joined, holes,
  pockets and cutouts taken away), and the scan's points (all vertices up to 200,000, otherwise 200,000 evenly spaced by vertex index) are measured against
  it: `details.check` has `points`, `mean_mm`, `rms_mm`, `median_mm`, `p95_mm`, `max_mm` and `within_tolerance` (the share of
  points within the plate's tolerance). Unions and cuts combine by min and max, which is exact on faces and
  slightly low near edges where features meet. When 5 % of the scan is more than three tolerances off, the
  report says so (`scan_deviation`), pointing at what was dropped. Refuse the proposed plate when
  the median sampled vertex distance exceeds both ten times the deviation tolerance and one tenth
  of the largest plate dimension. This is a gross mismatch check over sampled vertices. The user
  can still open the original file as a mesh.
- **Parameters.** `length`, `width`, `thickness`, `hole<i>_diameter`, `boss<i>_diameter`, `boss<i>_height`,
  `pocket<i>_depth`, `pad<i>_height` are named parameters (to a micrometre). Pockets hang from `thickness`, so
  changing the thickness keeps their depth.
- **Loss report.** Everything not rebuilt is an item, never a guess: `slanted_face`, `tilted_round`,
  `round_outside`, `hole_internal`, `boss` (not standing on an upward face), `round_outline`, `pocket_rotated`,
  `pad_rotated`, `undercut`, `below_base`, and simplifications `outline_rectangle`, `outline_faceted`, `cutout_not_round`, `pocket_rectangle`,
  `pad_rectangle` (a face covering less than 98 % of its rectangle). `details` carries the plate, holes, bosses,
  pockets and pads with their measured sizes and fit quality, and the frame that maps the design back onto the scan.

Known answers also: a Ø6 hole countersunk 90° to Ø12 (11,604.159 mm³), a plate with four top edges bevelled
2 x 1 (11,688.254 mm³) and the L-plate with two bevelled edges (12,461.305 mm³), all equal to SolidCraft's volume;
the countersink and bevels follow `thickness` when it is edited. Fillets: a plate with four top edges rounded R2
(11,708.643 mm³) and the L-plate with three rounded R1.5 (12,516.510 mm³), exact, and exact after a thickness edit. Known answers: an L-shaped plate (fixtures/known/l-plate.stl, 60 x 20 plus 20 x 20, 8 mm thick, Ø6 hole) rebuilds
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
