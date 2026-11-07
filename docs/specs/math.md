# Math core

Status: R1 spec. Feature: `math.core`.

Small, exact-enough linear algebra the fits share, in `rulercraft-mesh::math`, all `f64`:

- 3-vector helpers: dot, cross, length, normalise (refusing zero length or non-finite components),
  add/scale. Normalisation scales by the largest component first, so nonzero finite subnormal and
  extreme directions remain unit length.
- Symmetric 3x3 eigen decomposition: eigenvalues ascending with orthonormal eigenvectors (Jacobi
  rotations are fine and simple). Repeated eigenvalues return some orthonormal basis of the space.
- Centroid and covariance of a set of points (two-pass for accuracy on large coordinates far from 0).
- A small dense least-squares / Levenberg-Marquardt helper for up to ~7 parameters, used by the sphere
  and cylinder refinements.
- `atan2`-based unsigned and signed angle helpers that operate on normalized directions and behave at
  ±π across finite input scales.
- The least-squares solver counts a point where no step lowers the cost as converged only when the
  residual is orthogonal to every Jacobian column (|cos| ≤ 1e-6): a scale-free test, so a fit behaves
  the same in mm and in inches. A stall anywhere else is reported as not converged.

Checks: eigen of diag(3,1,2) gives 1,2,3 with axis vectors; a random rotation R·diag·Rᵀ is recovered
within 1e-12; covariance of points on a plane at z = 1e6 has its smallest eigenvector along z within
1e-9; LM fits a known circle exactly from a bad start.
