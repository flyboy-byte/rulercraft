# Fit cylinder

Status: R1 spec. Feature: `fit.cylinder`.

- Input: positions, normals, a selection, outlier cut-off as for planes.
- Axis start: the direction most perpendicular to the selection's vertex normals (smallest eigenvector
  of the normals' covariance). If that is unstable (a nearly flat selection), fall back to a robust
  sampled estimate; if still nothing, refuse: the surface curves too little to tell from a plane.
- Then fit the circle of the points projected onto the plane perpendicular to the axis, and refine
  all five parameters (axis direction, axis point, radius) geometrically, with clipping as for planes.
- Output: axis point at the middle of the measured length, unit axis, radius, diameter, length (span
  of the kept points along the axis), angular coverage in degrees (360 minus the widest gap), sigma,
  form error, residual min/max, points used/offered.
- Options carried with the element, not folded into the measurement: an assumed (designed) diameter,
  and how far past each end the cylinder is drawn.
- Refusals: too few points; no axis; the fit diverges (non-finite or non-positive radius).

Note: angle computations near ±π must use a well-behaved atan2, and coverage
on a partial arc depends on finding the largest gap after sorting angles, including the wrap-around gap.

Checks: a full tube of radius 10 gives radius 10 within 1e-5 and the axis within 1e-6; a 120° arc gives
coverage near 120°; a tilted axis (1,2,3) is recovered within 1e-4; a flat patch is refused.

## Implementation choices (2026-11-07)

- `fit_cylinder` takes positions, optional per-vertex normals, selected vertex ids, and options.
  Defaults: no clipping, no assumed diameter, zero end extensions; at most 1 million selected
  points, 100 refinement iterations, 10 clipping rounds and 8 axis starts. All limits are caller
  parameters. Invalid limits, indices, normals, dimensions and non-finite data refuse.
- At least six points are needed. A position covariance eigenvalue ratio below 1e-10 refuses
  near-planar data before optimization; it cannot determine a finite cylinder with axial span.
- Starts: least normal-covariance direction when stable, then sampled cross-products of normals
  (at most 16 evenly spaced samples), then position covariance axes. Each start fits a projected
  circle algebraically; the best radial-residual start is refined with five geometric parameters.
  Missing/zero normals use the position starts. Nearly parallel normal pairs are skipped.
- Axis sign is canonical (largest-magnitude component positive). Axis point is at the midpoint
  of the kept points' axial span. Residuals are signed radial distance minus radius; sigma is RMS.
  A fitted axial span below radius * 1e-8 refuses. Numerical convergence failure refuses.
- Clipping refits points within k times the all-points fit's RMS radial error (fixed for every
  round, as for planes), k = 1, 2 or 3, bounded by max_rounds.
  A step that keeps fewer than six points or becomes degenerate stops at the previous fit.
- Assumed diameter must be positive and extensions nonnegative; they are returned unchanged,
  never used to alter the measured fit. Coverage is 360 minus the widest sampled angular gap,
  including wrap-around, so a finite full ring reports 360 minus one sample spacing.
- These starts are deterministic, not a global optimization guarantee; severe outliers and very
  shallow arcs may refuse or converge to a different local optimum. No automatic surface selection.

- Clipping stops when RMS is below sqrt(3) * f32 epsilon * maximum absolute input coordinate.
  Below that conservative coordinate quantization bound, repeated clipping would discard points
  according to f32 storage roundoff rather than surface error. Fit tolerances remain unchanged.
