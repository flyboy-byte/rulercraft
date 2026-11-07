# Fit plane

Status: R1 spec. Features: `fit.plane`, later `fit.robust`.

- Input: positions, a selection, and an outlier cut-off: none, or k-sigma with k in {1, 2, 3}
  (default 3).
- Method: total least squares (the plane through the centroid whose normal is the smallest-eigenvalue
  eigenvector of the covariance). With a cut-off, repeat: drop points farther than k·sigma, refit,
  until the kept set stops changing or a few rounds pass.
- Clipping loop: fit all offered points; then keep every offered point within k·sigma of the current
  plane and refit; stop when the kept set no longer changes, sigma is 0, or after 10 rounds. If a round
  would keep fewer than 3 points, the previous fit stands.
- Limit: as for spheres (fit-sphere.md), clipping handles points somewhat off the face, not a few points
  far away with enough leverage to tilt the whole fit; that is `fit.robust` (R3).
- Output: centre (centroid of the kept points), unit normal, an in-plane basis (u along the direction
  the kept points spread most, v = normal × u), the span of the kept points along u and v (max minus
  min, mm), sigma (RMS signed distance of the kept points), form error (max minus min signed distance
  of the kept points), signed min and max, points used, points offered, rounds taken.
- Normal orientation: with vertex normals given, the side the selected vertices' normals mostly face
  (sum of dot products ≥ 0), so "outward" means something on a closed mesh. Without normals: the
  direction whose largest-magnitude component is positive (deterministic, but not "outward").
- Refusals: fewer than 3 points; an index past the vertex count; non-finite positions; all points on
  one line or at one point (the second-largest spread is below 1e-12 of the largest).

Checks: points on z = 5 with noise σ=0.01 give normal (0,0,±1) within 1e-3 and sigma near 0.01; a cube
face fits exactly with sigma below 1e-6; adding far outliers with a 3-sigma cut-off leaves the plane
unchanged within 1e-4.

## Robust fit (2026-11-07)

`fit_plane_robust(positions, normals, selection, options)` handles what clipping cannot: far outliers with
enough leverage to tilt the ordinary fit. It tries `iterations` (default 500, at most 100000) planes
through three random selected points, counts the points within `threshold` (default 0.1 mm) of each, keeps
the plane with the most, and fits the ordinary total least squares plane (no clipping) through exactly
those points. The sampling is a fixed sequence from `seed`, so a run is repeatable. Returns the fit and
the agreeing vertex ids. Refuses: a threshold that is not positive and finite, 0 or too many iterations,
fewer than 3 points, bad indices, and a selection with no three points making a plane that at least
3 points agree with. Check: 300 points on z = 5 plus 100 far one-sided outliers tilt the ordinary fit
off 1 in normal z, while the robust fit keeps normal z within 1e-4 and keeps no outlier.
Not done: robust sphere and cylinder seeding, and CLI/MCP exposure.
