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

## API details (2026-11-07)

- `MeshGraph::build(mesh, limits)` preserves the mesh's vertex and triangle ids; MeshBuilder is
  responsible for welding at import. Limits cap input vertices and triangles (50 million each by
  default). Non-finite positions, invalid indices and repeated triangle corners refuse with the
  vertex or triangle id. Zero-area triangles remain in topology but have no triangle normal.
- Edges are sorted undirected vertex pairs; incident triangles and vertex neighbours are sorted.
  Components include isolated vertices and list vertex and triangle ids. Component order follows
  the lowest vertex id. Normals use f64 arithmetic on the mesh's millimetre positions.
- Boundary edges are exposed separately. A boundary component is reported as a loop only when
  every vertex has boundary degree two; branched boundary components remain visible as edges.
  Each loop lists vertices without repeating its start. Closure means no boundary edges, even
  when a mesh is non-manifold. Non-manifold edges are counted, not refused.
- `crease_edges(angle)` accepts finite radians from 0 through pi; the convenience default is
  pi/6 (30 degrees). Only edges with exactly two nonzero triangle normals qualify, using strict
  greater-than comparison. Reversed winding can therefore create a crease; no winding repair.
