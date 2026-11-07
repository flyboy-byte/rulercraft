# Mesh model

Status: R1 spec. Feature: `mesh.graph`.

The in-memory mesh every other feature uses, built once after import.

- Vertex normals: area-weighted average of the adjacent triangle normals, normalised. A vertex whose
  triangles cancel out (zero length) gets no normal and is skipped by normal-based filters.
- Adjacency: for each vertex, its neighbouring vertices and triangles; for each edge, its one or two
  triangles. An edge with more than two triangles is non-manifold and counted.
- Connected components (by shared vertices), with their sizes.
- Open boundary loops (edges with a single triangle), so "is this mesh closed" is answerable.
- Sharp edges: an edge whose two triangle normals differ by more than a threshold angle (default 30°)
  is a crease; used by segmentation and display.
- All of it is computed in bounded time (linear in mesh size, aside from sorting).

Checks: a cube has 8 vertices, 18 edges, 12 triangles, 1 component, 0 boundary loops, 12 crease
edges at 30°; a single open square has one boundary loop of 4 edges.
