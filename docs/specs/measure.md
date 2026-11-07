# Measure

Status: R1 spec (outline for thickness). Features: `measure.relations`, `measure.thickness`.

Relations between fitted elements, in mm and degrees:

- plane-plane: angle; when near-parallel (under 1°), distance between them measured at the first
  plane's centre.
- point/sphere centre to plane: signed distance.
- point to point, centre to centre: distance and its x/y/z components.
- cylinder axis to plane: angle; axis to axis: angle and closest distance (parallel case handled).
- cylinder to cylinder, parallel: centre-line distance (hole spacing).
Each result says which elements it used and, for near-degenerate cases, which convention applied.

Thickness (R3, outline): distance through the material along the inward normal to the opposite wall.

## Wall thickness (2026-11-07)

`measure::thickness::wall_thickness(mesh, outward_normals, selection, options)` measures each selected
vertex along the negative of its supplied outward normal. Normals can come from MeshGraph. Returns
samples in selection order (duplicates preserved), each with vertex id and optional distance in mm.
The distance is the nearest positive intersection with an opposite, outward-facing triangle, excluding
triangles incident on the starting vertex. Missing hits are `None`, never zero or an invented value.

The mesh must have consistent outward winding; this is directional thickness, not a shortest-distance
or medial-axis method. At sharp corners averaged vertex normals give oblique paths. Open meshes,
reversed normals, gaps and max-distance cutoffs may give missing hits. It does not certify a closed
solid or repair winding. Degenerate target triangles are ignored.

Options: default maximum 5 million selected-vertex/triangle tests, ray distance at most 1,000,000 mm,
self-hit exclusion distance 1e-6 mm. Limits checked before work; distance/exclusion must be finite,
positive and max distance greater than exclusion. Any invalid index, nonfinite position, mismatched
normal count, or invalid selected normal refuses with the vertex/triangle context. Empty selection
succeeds with no samples. Triangle barycentric coordinates allow 1e-12 edge roundoff; directions with
normal cosine <=1e-12 are parallel or not outward-facing and are ignored.

MCP `thickness` selects vertices with the existing filters and returns a page of directional samples.
The selected indices are sorted; `offset` defaults to 0 and `limit` defaults to 256, maximum 1024.
Results contain `total`, `offset`, `returned`, `truncated` and `samples` (vertex id and distance, or
null when there is no hit). The page is checked against the engine's 5 million triangle-test limit.
`max_distance` may lower the engine default of 1,000,000 mm; invalid limits and selections are refused.
The mesh is unchanged. CLI exposure and acceleration for dense scans remain separate work.

Axis-axis distances use the closest points on infinite lines, including small genuine tilts.
For unit axes with cross-product magnitude <=1e-12, floating-point directional roundoff is treated
as parallel and the first axis point is measured to the second line. This numerical cutoff replaces
the earlier 1-degree parallel approximation for axes (checkpoint #32, 2026-11-07). Small nonparallel
angles can place the closest points far outside the measured surface; the result is not spacing at a
specified height. Plane-related near-parallel conventions retain their documented 1-degree threshold.
