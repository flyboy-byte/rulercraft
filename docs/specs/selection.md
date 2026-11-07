# Selection

Status: R1 spec. Feature: `select.region`.

Marks the vertices a fit runs on.

- **Box:** every vertex inside an axis-aligned box (min/max, mm, inclusive). Refused when min > max
  on any axis.
- **Near:** every vertex within a radius of a point.
- **Facing:** optionally keep only vertices whose normal is within an angle (default 30°) of a given
  direction (any non-zero length). A zero direction is refused.
- **Explicit list** of vertex indices; an index past the vertex count is refused, naming it.
- Results are returned as a count, and optionally the indices (capped, evenly thinned, with the step
  stated) so a large selection does not flood an agent.
- A fit needs at least a handful of points (3 for a plane, 4 for a sphere, 5 for a cylinder); fewer is
  refused with how many were found.
