# Segmentation

Status: R1 spec for (a); outline for (b) and (c). Features: `segment.planes`, `segment.cylinders`,
`segment.strips`.

Lets a user or agent say "the top face" or "that hole" without marking vertices by hand.

## (a) Flat regions (`segment.planes`)

- Input: a mesh and its graph; options: largest angle between a triangle's normal and its region's plane
  (default 10°), largest distance of a triangle's corners from that plane (default 0.2 mm), smallest
  region area kept (default 0, keep all).
- Seeds: unassigned triangles in order of flatness: the largest angle to any edge-neighbour, smallest
  first, ties by larger area, then by index. Zero-area triangles are never seeds and never join.
- Growth: breadth-first across shared edges; a neighbour joins when both limits hold against the region's
  current plane. The plane starts as the seed triangle's and is refitted (total least squares on the
  region's vertices so far) whenever the region has doubled in triangle count since the last refit.
- After growing, a region is refitted and any triangle with a corner past the distance limit from that
  fit is handed back (free for later seeds), repeating until none strays. So every region lies within
  the distance limit of the plane it reports. Without this a region creeps along a gentle slope (found
  2026-11-07 on a 5° ramp: 0.29 mm off its own plane). A region may still take in the start of a gentle
  slope as long as it stays within the limits.
- Every triangle ends in at most one region. Regions below the area limit are dropped and their
  triangles left unassigned.
- Output: regions largest area first, each with its triangles, its vertices (sorted, unique), its area
  (mm²) and its plane fit (fit.plane, no clipping, oriented by vertex normals); and the count of
  unassigned triangles.
- A faceted round surface comes out as strips that each look flat; (c) joins them.

Checks: a cube gives 6 regions of 2 triangles, each with its face's normal and area; an L-shaped block
gives its 8 faces; a noisy plate (noise under the distance limit) is one region; a flat plate with a dome
on it keeps the dome out of the plate's region; refusals: none (an empty mesh gives no regions).

## (b) Round regions (`segment.cylinders`)

- Input: a mesh, its graph; options: largest angle between a triangle's normal and the cylinder's radial
  direction at its centroid (default 15°), largest distance of a triangle's corners from the cylinder
  surface (default 0.2 mm), largest radius accepted (default 10,000 mm, beyond that it is flat), smallest
  angular coverage kept (default 20°).
- Seeds: triangles whose largest angle to an edge-neighbour is between 0.5° and 45° (curving, not a sharp
  edge), in index order. A seed patch is the seed and every triangle within two edge-steps; it is kept only
  if fit.cylinder accepts the patch's vertices (with vertex normals), the radius is under the limit, and
  every patch corner lies within the distance limit.
- Growth: breadth-first across shared edges; a neighbour joins when its surface (corners, edge midpoints
  and centroid, not only corners: a large flat triangle's corners can sit on a big cylinder while its
  middle sags off it) is within the distance limit of the current cylinder and its normal is within the angle of the radial direction (either sign,
  so holes and bosses both work). The cylinder is refitted whenever the region has doubled.
- Trim: as for flat regions, refit and hand back triangles with a corner past the distance limit until
  none strays, so every region lies within the limit of the cylinder it reports.
- Regions below the coverage limit are dropped (their triangles freed). Output: regions largest area
  first, each with triangles, vertices, area and its cylinder fit; and the count of triangles in none.

Checks: an open tube is one region with its radius and axis; a block with one rounded edge gives the
fillet alone (radius, axis, about 90° coverage) and none of the flat faces; two separate tubes give two
regions; a cube gives none; every region stays within the limit of its cylinder.

## (c) Joining strips (`segment.strips`), outline

Neighbouring flat regions whose normals turn steadily about a common axis and whose fitted planes are all
tangent to one cylinder are one round surface; report it as one region with its cylinder fit.
