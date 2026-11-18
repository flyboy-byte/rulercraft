# Selection

Status: R1 spec. Feature: `select.region`.

Marks the vertices a fit runs on.

- **Box:** every vertex inside an axis-aligned box (min/max, mm, inclusive). Refused when min > max
  on any axis.
- **Near:** every vertex within a radius of a point.
- **Facing:** optionally keep only vertices that face a given direction (any non-zero length) within an angle
  (default 30°): by their own normal, or by any triangle touching them (`select::facing_any`, used by every
  command, the CLI, MCP and the app). Vertex normals are area-weighted averages of the triangles around a
  vertex, so on a sharp-edged CAD mesh a face whose vertices are all corners (a box side) has no vertex whose
  own normal faces it; its triangles do (2026-11-18; before, such a face could only be selected by box).
  `select::facing` keeps the normal-only rule for library callers. A zero direction is refused.
- **Explicit list** of vertex indices; an index past the vertex count is refused, naming it.
- Results are returned as a count, and optionally the indices (capped, evenly thinned, with the step
  stated) so a large selection does not flood an agent.
- A fit needs at least a handful of points (3 for a plane, 4 for a sphere, 5 for a cylinder); fewer is
  refused with how many were found.

Finite near queries compare unsquared distances to avoid overflow/underflow. Facing directions are
scaled before normalization so any finite nonzero direction, including subnormal values, works.
Facing compares the angle itself, including the boundary, so narrow cones remain selective near zero radians.
