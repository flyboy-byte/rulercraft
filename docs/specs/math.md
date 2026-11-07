# Math core

Status: R1 spec. Feature: `math.core`.

Small, exact-enough linear algebra the fits share, in `rulercraft-mesh::math`, all `f64`:

- 3-vector helpers: dot, cross, length, normalise (refusing zero length), add/scale.
- Symmetric 3x3 eigen decomposition: eigenvalues ascending with orthonormal eigenvectors (Jacobi
  rotations are fine and simple). Repeated eigenvalues return some orthonormal basis of the space.
- Centroid and covariance of a set of points (two-pass for accuracy on large coordinates far from 0).
- A small dense least-squares / Levenberg-Marquardt helper for up to ~7 parameters, used by the sphere
  and cylinder refinements.
- `atan2`-based angle helpers that behave at ±π.

Checks: eigen of diag(3,1,2) gives 1,2,3 with axis vectors; a random rotation R·diag·Rᵀ is recovered
within 1e-12; covariance of points on a plane at z = 1e6 has its smallest eigenvector along z within
1e-9; LM fits a known circle exactly from a bad start.
