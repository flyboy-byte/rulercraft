# Fit shapes: cone and circle

Status: R7 spec (2026-11-19). Feature: `fit.shapes`. Torus is not done yet.

## Cone

- Input: positions, optional normals (one per vertex; zero normals are ignored), a selection of at least 7 points,
  outlier clipping as for planes (the cut-off fixed from the fit of every point, at most 10 rounds).
- Starts: the direction the normals vary least along (a cone's normals all make one angle with its axis) when there
  are normals, then the points' three principal directions. For each start: the axis through the centroid, the
  radius a straight line in height along it (least squares). Each start is refined geometrically and the one with
  the smallest sum of squares wins.
- Model: a point on the axis, the unit axis, the radius there and its slope along the axis (tan of the half-angle).
  The residual of a point is its signed distance from the surface, outside positive:
  (ρ − (r + t·h)) / √(1 + t²), with h along the axis and ρ across it. All six parameters are refined with the
  shared Levenberg–Marquardt solver (finite-difference Jacobian).
- Output: the axis pointing from the narrow end to the wide end, a point on it at the middle of the kept points' axial
  span, the half-angle (the CLI and MCP also give the included angle, twice it), the radius there, the diameters at
  both ends of the span, the apex, the length, sigma, form error, residual minimum and maximum, points used, offered,
  rounds.
- Refusals: fewer than 7 points; no start converges; a half-angle under 0.5° ("fit a cylinder") or over 85° ("fit a
  plane"); a non-finite result or the apex inside the measured span.

## Circle

- Input: positions, a selection of at least 3 points, clipping as for planes.
- The plane of the points (fit.plane, no clipping), then the circle of the points projected into it: algebraic
  (Kasa) start, then geometric refinement of centre and radius. Clipping drops points by their radial distance.
- Output: centre, the plane's normal, radius, diameter, angular coverage (360° minus the widest gap, the wrap-around
  gap included), sigma, form error, residual minimum and maximum, the plane's RMS (flatness), used, offered, rounds.
- Refusals: fewer than 3 points; points on a line.

## Commands

`FitKind::Cone` and `FitKind::Circle` in the command layer, `cone` and `circle` on the CLI (`fit`, `measure`) and the
MCP server. A known diameter is refused for both. In relations a cone and a circle act through their axis line (a
circle's: through its centre along its normal), as a cylinder does.

## Known answers

tests/fit_shapes.rs: an exact 30° cone with its apex at (1, 2, 3) on a tilted axis gives its half-angle within 1e-6°,
apex within 1e-5 mm and end diameters within 1e-5; a 90° countersink-shaped half cone with 0.01 mm noise and every
25th point 0.5 mm off gives 45° within 0.2° with the outliers dropped; a cylinder is refused. A tilted circle of radius
7 gives radius and centre within 1e-5 (f32 positions); a 90° arc of 40 points gives coverage 87.75°; with 0.02 mm noise
radius within 0.005. Through the CLI (tests/measure.rs): the countersink SolidCraft made gives 90.0°, Ø6 to Ø12, apex
at z = 4 within 1e-4; the hole block's Ø3 rim circle within 1e-5. Wrong-value runs: no clipping (the outlier test
fails), no wrap-around gap (the coverage test fails).
