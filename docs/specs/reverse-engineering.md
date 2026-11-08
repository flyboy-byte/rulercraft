# Reverse engineering

Status: *Outline*. Feature: `re.features`.

Higher-level guesses on top of segmentation: hole patterns (equal diameters on a circle or grid),
symmetry planes, rounding measured values to likely design values (5.98 → 6.00) with the evidence for
each guess, so Claude can propose a feature tree that a person accepts or edits.

## Hole patterns and design values (2026-11-07)

`reverse::hole_patterns(holes, options)` takes fitted round features (name, a point on the axis, unit axis,
diameter), groups those with the same diameter (0.05 mm) and parallel axes (2°), and reports a group of 3
or more only when the whole group lies on one line or one circle (within 0.1 mm). A line reports its
members in order, mean spacing, whether every gap is within 0.1 mm of it, and the worst offset. A circle
reports members in order, centre, radius, the smallest step between neighbours in degrees and the worst
offset; it is reported only when every gap, including the closing one, is a whole number of that step (a
bolt circle, possibly with holes missing), because any three holes lie on some circle. A group that is
only partly a pattern is left alone rather than split by guesswork.

`reverse::design_guesses(measured, tolerance)` offers, for each family of round numbers (whole, half and
tenth of a millimetre, then eighth, sixteenth and thirty-second of an inch), the nearest round value when
the measurement is within the tolerance, with the error as the evidence (5.98 gives 6.0 and error -0.02).
Refused: a non-finite value or a tolerance of 0 or less; a round value of 0 is never offered.

Checks: a six-hole bolt circle with an odd-size hole present gives one circle with the right centre,
radius and 60° step; arcs of three holes are circles and irregular ones are not; a row of five gives
spacing 15 and an uneven row is flagged; turning the part changes no number; empty, tiny, zero-axis,
NaN and coincident inputs give nothing and do not panic. Four wrong-value runs (irregular circles
allowed, diameters ignored, spacing check dropped, guess tolerance ignored) each failed a test.
Not done: grid patterns, grouping a partial pattern out of a larger group, and CLI/MCP.

## Symmetry planes (2026-11-07)

`reverse::symmetry_planes(mesh, tolerance)` tries the three planes through the vertex centroid whose
normals are the principal axes of the vertex cloud. Every vertex is reflected through the plane and its
distance to the mesh surface measured (exact, as in deviation.md). Each plane reports RMS and largest
distance, and is called a symmetry plane when the largest is within the tolerance. Limits: only those
three candidate planes are tried; a part with nearly equal spread along two axes (a cube, a round part)
has no well-defined principal axes there, so a true plane can be missed; vertices are sampled, not
surface points. Refused: a tolerance that is not finite and above 0, fewer than 4 vertices, no triangles.
Checks: a plate with a centred block on one end is symmetric across its width only, with the plane at the
right place, and the verdict is unchanged after turning and moving the part. Three wrong-value runs
(mirror not doubled, tolerance ignored, centroid dropped) each failed two tests.

## Grid patterns (2026-11-07)

A group of 4 or more equal parallel holes that is neither a line nor a circle is tried as a lattice: the
first lattice direction is the vector from the first hole to its nearest neighbour, the second is the
nearest vector not along the first (the directions need not be square). Every hole must sit within the
position tolerance (0.1 mm) of an integer combination of the two vectors, and no two holes may share a
lattice position. A grid needs at least 2 rows and 2 columns. It reports members row by row, columns, rows,
both spacings, the angle between the directions (90 for a rectangular grid), whether every position inside
the rows and columns holds a hole (`full`), and the worst offset. A square of four holes is reported as a
circle, since it is one. Checks: a 4 x 3 grid at 10 x 15 mm; a grid with a hole missing is `full: false`;
a hole pulled off its position makes it no grid; two holes on one position make it no grid; a turned grid
reports the same spacings. Three wrong-value runs (full always true, position tolerance ignored, duplicate
positions allowed) each failed a test.

At extreme coordinate ranges, a row is omitted when displacement rounding may exceed the requested
position tolerance or the computed member order contradicts adjacent projections. Row ordering uses the
largest direction component to retain distinguishable nearby positions across large coordinate ranges. Grid indices and
dimensions that cannot be represented are omitted; an unrepresentably large cell count is not full.
