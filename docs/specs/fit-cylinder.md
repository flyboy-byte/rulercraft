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
