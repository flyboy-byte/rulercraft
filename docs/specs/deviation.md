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
The MCP server exposes comparison and patches; the CLI exposes unaligned deviation as described below.

## CLI unaligned deviation

`rulercraft deviation <measured> <reference>` compares the meshes in their existing coordinates; it does
not align them. Source units default to mm and can be set independently with `--units` and
`--reference-units`. `--tolerance` is a nonnegative finite millimetre value (default 0.1). JSON returns
global engine `stats`, `total`, `offset`, `returned`, `truncated`, and `samples` (vertex index and
signed distance in mm). `--offset` defaults to 0 and `--limit` to 256, with a maximum of 1024.
Statistics and engine work cover all measured vertices even when samples are paged; paging bounds output, not computation. The reference winding determines
sign; the output states that no alignment was applied. Invalid inputs refuse cleanly.
