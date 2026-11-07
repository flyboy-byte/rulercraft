# Fit sphere

Status: R1 spec. Feature: `fit.sphere`.

- Input: positions, a selection, outlier cut-off as for planes, optionally a known diameter (then only
  the centre is fitted).
- Method: algebraic least-squares sphere for a start, then geometric refinement (minimise distance to
  the surface, Gauss-Newton or Levenberg-Marquardt), with the same clipping loop as planes.
- Output: centre, radius, diameter, sigma, form error, min/max radial residual, points used/offered,
  and how much of the sphere the points cover (solid-angle fraction or degrees of arc).
- Refusals: too few points; points nearly coplanar (no stable sphere); a known diameter that differs
  from the free fit by more than a stated limit (the surface is probably not that ball).

Checks: points on a cap of a radius-12.7 sphere give radius 12.7 within 1e-4; a full sphere gives its
centre within 1e-6.
