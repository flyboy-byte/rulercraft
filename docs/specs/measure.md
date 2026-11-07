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

## Relations API and CLI (2026-11-07)

- `measure(MeasureRequest)` accepts two named elements: point, sphere centre, plane (centre and
  normal), or cylinder (axis point and axis direction). Directions are normalized; zero/non-finite
  directions and non-finite positions/results refuse. There is no input-size-dependent loop.
- Direction angles are unsigned acute angles: opposite plane normals/axis directions are equivalent.
  Axis-plane angle is 0 for an axis parallel to the plane and 90 degrees for perpendicular.
  Engine angles are radians; CLI JSON uses `angle_deg`.
- Plane separation is unsigned distance from the first centre to the second plane, only when the
  acute angle is strictly below 1 degree. Otherwise distance is absent. Point-plane distance is
  signed along the plane normal, regardless of argument order. Sphere means its centre, not surface.
- Point/centre pairs return second minus first x/y/z components and their Euclidean length.
  Axis-axis returns closest infinite-line separation. When cross-direction length is <= 1e-12,
  use the parallel approximation: distance from the second origin to the first axis. This convention
  is returned explicitly. Cylinder radius and finite length do not enter centre-line measurements.
- Each result carries first/second element ids, kind and a named convention. Unsupported pairs
  (such as point-axis) refuse. Plane/axis arguments can be in either order.
- CLI: `rulercraft measure first.json second.json`. Each file is a geometry descriptor with an
  optional `id` (default filename), or a prior CLI fit output containing `fit`. Supported descriptors:
  `{"kind":"point","position":[x,y,z]}`, `{"kind":"sphere","center":[x,y,z]}`,
  `{"kind":"plane","center":[x,y,z],"normal":[x,y,z]}`,
  `{"kind":"cylinder","axis_point":[x,y,z],"axis":[x,y,z]}`. Lengths are already mm.
  JSON input is limited to 1 MiB per element. Errors exit 2 with no stdout.
