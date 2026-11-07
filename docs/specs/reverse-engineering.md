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
Not done: symmetry planes, grid patterns, grouping a partial pattern out of a larger group, and CLI/MCP.
