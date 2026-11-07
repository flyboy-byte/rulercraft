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
