# Segmentation

Status: R1 spec for (a), (b), (c), and their CLI/MCP front ends. Features: `segment.planes`,
`segment.cylinders`, `segment.strips`, `segment.frontends`.

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

## (c) Joining strips (`segment.strips`)

A coarsely faceted hole, boss or fillet (a CAD export with few segments) has facet middles that sag past
(b)'s distance limit (a 12-sided radius-6 tube sags 0.2 mm), so (b) misses it and (a) gives one flat
region per facet. `join_strips` joins those back into one round surface.

- Input: a mesh, its graph and its flat segmentation from (a); options: largest angle between
  neighbouring strips' normals (default 40°: a prism of nine sides or fewer stays flat, ten or more is
  round), largest distance of a strip's vertices from the cylinder (default 0.2 mm; vertices only, since
  facet middles sag inside the true surface by design), largest radius (10,000 mm), fewest strips (3),
  smallest angular coverage (20°).
- Neighbours: two flat regions that share a manifold edge and whose normals are within the turn limit.
- Seeds: flat regions in their segmentation order (largest first), each paired with each free
  neighbour. Quick test first: the pair's axis is across their two normals, and seen along it their
  vertices must lie on one circle within the distance limit (algebraic circle fit). Then fit.cylinder on
  the pair's vertices (radius under the limit).
- Growth: breadth-first over neighbours; a strip joins when all its vertices are within the distance
  limit of the current cylinder. After each pass the surface is refitted on all its strips and grown
  again, until a pass adds none (a fit from two noisy strips extrapolates poorly round the circle).
- Kept when it has at least the fewest strips and the coverage; otherwise its strips stay free for
  later seeds. A strip is in at most one surface. Output: surfaces largest area first, each with its
  strips (indices into (a)'s regions), triangles, vertices, area and cylinder fit on its vertices.
- Meant for tessellated CAD meshes. On a scan (the private vase, 43,613 flat regions) it runs in about
  1.5 s but reports some 2,000 small low-coverage patches of the curved body; use (b) for scans.
- Known limit: vertex noise near half the distance limit (±0.12 mm on a radius-8, 14-sided tube) splits
  the surface into several with wrong radii, because a wrong cylinder through a few noisy strips still
  holds them. Up to ±0.08 mm it stays one surface.

Checks: a 12-sided tube is one surface of 12 strips with its radius and axis; a fillet in four 22.5°
steps gives the fillet alone (radius 3, about 90°) and no flat face; two coarse tubes give two surfaces;
a 14-sided tube with ±0.08 mm noise on every vertex is one surface; 3-, 4-, 6- and 8-sided prisms
(corners all on a circle) stay flat; a bent plate of three wide 20° panels (creases on a circle, inner
grid points off it) is not round; an 18° arc of three strips is dropped by the coverage limit and kept
without it; every surface's vertices lie within the limit of its cylinder. Opt-in: the private scan
joins in under 30 s.

## CLI and MCP front ends (`segment.frontends`)

- `rulercraft segment <mesh> [--kind planes|cylinders|strips] [--units mm|cm|m|in]` prints one compact JSON summary. The kind defaults to `cylinders`, which is the useful default for scans; joined strips are opt-in for tessellated CAD meshes because a scan can produce thousands of small low-coverage patches.
- The MCP `segment` tool uses the currently open mesh and accepts optional `kind` with the same values and default.
- Plane and cylinder modes call `segment_planes` and `segment_cylinders` with their default limits. Strip mode first runs `segment_planes`, then `join_strips` with default limits.
- Output has `kind`, `regions`, and `unassigned`. Each region summary contains its triangle and vertex counts, area, and the corresponding plane/cylinder fit fields; strip summaries also contain the number of joined strips. Strip `unassigned` counts triangles absent from returned surfaces. Full vertex/triangle id lists are not returned.
- Unknown kinds and import failures are refused cleanly. The CLI uses the existing bounded mesh loader; MCP operates on the current already-loaded mesh.

The front ends do not expose segmentation tuning options in this first pass; all three modes use the existing library defaults.
