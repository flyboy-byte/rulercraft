# Deviation

Status: *Outline*. Feature: `compare.deviation`.

Per-vertex signed distance from a mesh to a reference mesh (closest point on the reference surface,
sign from the reference normal), with statistics (mean, RMS, percentiles, max inside/outside) and
patches of vertices beyond a tolerance.

## Library (2026-11-07)

`rulercraft_mesh::deviation::deviation(mesh, reference, tolerance)` gives every vertex of `mesh` its
signed distance to `reference`, plus count, mean, RMS, largest outside/inside, median and 95th-percentile
absolute distance, and how many are beyond `tolerance`. Distance is exact (bounding-box tree, checked
against brute force on random triangles). Sign follows the closest reference triangle's face normal, so
the reference should have consistent outward winding; beside an edge or corner the sign may follow either
neighbour, but the size never changes. An empty reference or a bad tolerance is refused. Percentiles use
nearest rank. Alignment before comparing is `align.*`.

`deviation::patches(graph, distances, tolerance)` groups vertices beyond the tolerance into patches:
vertices joined by a mesh edge and on the same side (outside or inside). Each patch gives its vertex ids,
side, mean signed distance and largest absolute distance. Largest first, ties by lowest vertex id. Vertices
with a non-finite distance are never in a patch; a tolerance that is not a finite number of 0 or more is
refused. Checks: two bumps and a touching opposite-side vertex give three patches in order; each refusal.
Not done: CLI/MCP exposure.
