# Integration audit corrections (2026-11-07)

Codex, checkpoint.r1-exit #32. All five integration PRs (#26, #27, #28, #29, #31) merged with merge
commits after local gates; their hosted gates and main runs are green. No dependencies added here.

Four focused problems reproduced before fixes:
- Near selection at centre (1e200,0,0), radius 1e199 selected the origin: squared distance and radius
  both overflowed to infinity. At centre (1e-200,0,0), radius zero, underflow also selected it.
  Stable vector length now compares distances directly.
- Facing direction (0,0,1e-320) selected nothing despite pointing along +z. Scaling direction components
  before normalization avoids reciprocal overflow. Zero and nonfinite directions still refuse.
- Axes at 0.5 degrees, starting at (0,0,0) and (10,0,0), intersect as infinite lines but reported
  distance 10. The 1-degree parallel approximation was used without saying it was approximate.
  True infinite-axis distance now applies above a 1e-12 cross-product magnitude; below that numerical
  precision cutoff it uses parallel spacing. The initial exact-zero-only fix failed the known MCP
  corpus because fitted analytically parallel axes carry solver roundoff. The explicit 1e-12 cutoff
  addresses that degeneracy; all 41 fixed corpus measurements pass without changing tolerances.
- A private capture expected integer 10^1000 raised OverflowError and a traceback. It now refuses
  with the same finite-number error as other invalid numeric captures.

Evidence: fixed-answer regressions, rigid motion and pair-order checks, original four-part corpus over
CLI and MCP, and private scan comparison. Reader mutation/prefix checks now include ZIP 3MF (about
0.21 s); project prefix/invalid-byte checks run in about 0.02 s. No panic found in these bounded sets.

Remaining: the public math-helper range finding, the pre-existing clipping-cap finding, undo/redo,
segmentation and project tools in MCP, dense thickness queries. The private scan is not a part with
published hole/face dimensions; Logan's R1 exit condition and printed/caliper evidence remain open.
