# Fit sphere

Status: R1 spec. Feature: `fit.sphere`.

- Input: positions, a selection, outlier cut-off as for planes, optionally a known diameter (then only
  the centre is fitted).
- Method: algebraic least-squares sphere for a start, then geometric refinement (minimise distance to
  the surface, Gauss-Newton or Levenberg-Marquardt), with the same clipping loop as planes.
- Clipping loop as for planes (fit-plane.md), on the radial residual |p − centre| − radius.
- Limit: clipping removes points somewhat off the surface (scan spikes, a little of a neighbouring face).
  A few points far from the surface can pull the least-squares sphere until they sit closer to it than
  the good points, and then clipping cannot find them. Robust seeding is `fit.robust` (R3); until then,
  mark the surface without far-off points.
- Output: centre, radius, diameter, sigma (RMS radial residual), form error (max minus min residual),
  min/max residual, points used/offered, rounds, and coverage as a cap angle in degrees: the largest
  angle between the points' mean direction from the centre and any kept point's direction (90° is a
  hemisphere, 180° the whole ball). With a known diameter, also the diameter the same points fit freely.
- Refusals: fewer than 4 points; points (nearly) coplanar, i.e. their least spread is below 1e-10 of the
  largest; the fit fails to converge or gives a non-positive radius; a known diameter that differs from
  the free fit by more than max(1 mm, 10 %) (the surface is probably not that ball).

Checks: points on a cap of a radius-12.7 sphere give radius 12.7 within 1e-4; a full sphere gives its
centre within 1e-6.
