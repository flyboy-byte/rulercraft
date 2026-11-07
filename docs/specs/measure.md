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

This feature adds the bounded engine API only. CLI/MCP thickness tools and acceleration for dense
scans are later front-end/performance work. Large queries refuse rather than quietly subsampling.
