# Fit plane

Status: R1 spec. Features: `fit.plane`, later `fit.robust`.

- Input: positions, a selection, and an outlier cut-off: none, or k-sigma with k in {1, 2, 3}
  (default 3).
- Method: total least squares (the plane through the centroid whose normal is the smallest-eigenvalue
  eigenvector of the covariance). With a cut-off, repeat: drop points farther than k·sigma, refit,
  until the kept set stops changing or a few rounds pass.
- Output: centre (centroid of the kept points), unit normal, an in-plane basis (u, v), the extent of
  the kept points along u and v, sigma (RMS distance of kept points), form error (max minus min signed
  distance), signed min and max, points used, points offered.
- Normal orientation: pointing to the side the selected vertices' normals mostly face, so "outward"
  means something on a closed mesh.
- Refusals: too few points; all points on a line (no unique plane).

Checks: points on z = 5 with noise σ=0.01 give normal (0,0,±1) within 1e-3 and sigma near 0.01; a cube
face fits exactly with sigma below 1e-6; adding far outliers with a 3-sigma cut-off leaves the plane
unchanged within 1e-4.
