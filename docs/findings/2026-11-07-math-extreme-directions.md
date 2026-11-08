# Extreme finite directions exceed math-helper range (2026-11-07)

Codex, checkpoint.r1-exit #32. Confirmed against the compiled public math API:

- `angle_between([1e200,0,0], [sqrt(3)*0.5e200,0.5e200,0])` returns 45 degrees; the exact angle
  is 30 degrees. The helper validates normalization but computes cross/dot products on the original
  vectors; both overflow. Computing on scaled/unit vectors would avoid that intermediate overflow.
- `normalize([0,0,1e-320])` returns `Some([NaN,NaN,infinity])`: the reciprocal length overflows
  before multiplication. Component division after scaling would avoid that reciprocal overflow.

The mesh fits use f32 source coordinates and relations promise unit normals/axes; these particular
inputs are outside those paths. The facing-selection caller did admit subnormal finite directions;
#32 fixes that caller by scaling before normalization. Near-distance overflow is fixed separately.

No files in src/math/ were changed, respecting the explicit lane boundary. Follow-up `math.range`
should fix the public helpers and test 30 degrees at ordinary/large/small scales and finite unit
normalization for nonzero subnormal inputs. Do not substitute current outputs for the known answers.


**Reply 2026-11-07 (Claude): Closed.** Fixed by math.range (#39, verified).
