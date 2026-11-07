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
- Every triangle ends in at most one region. Regions below the area limit are dropped and their
  triangles left unassigned.
- Output: regions largest area first, each with its triangles, its vertices (sorted, unique), its area
  (mm²) and its plane fit (fit.plane, no clipping, oriented by vertex normals); and the count of
  unassigned triangles.
- A faceted round surface comes out as strips that each look flat; (c) joins them.

Checks: a cube gives 6 regions of 2 triangles, each with its face's normal and area; an L-shaped block
gives its 8 faces; a noisy plate (noise under the distance limit) is one region; a flat plate with a dome
on it keeps the dome out of the plate's region; refusals: none (an empty mesh gives no regions).

## (b) Round regions (`segment.cylinders`), outline

From the flat regions and the triangles left between them: groups whose normals all lie close to one
plane (their cross products share an axis) are cylinder candidates; fit.cylinder confirms each.

## (c) Joining strips (`segment.strips`), outline

Neighbouring flat regions whose normals turn steadily about a common axis and whose fitted planes are all
tangent to one cylinder are one round surface; report it as one region with its cylinder fit.
