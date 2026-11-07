# Alignment

Status: *Outline*. Feature: `align.icp`.

Move a scan onto a reference: by datums (three planes, plane-line-point) and by ICP (point-to-plane),
reporting the rigid transform and the residual.

## ICP (2026-11-07)

`align::align_icp(scan, reference, start, options)` moves a scan onto a reference by a rigid transform
(a turn and a move, no scaling), point-to-plane. Each round pairs scan vertices (at most `max_points`,
evenly spaced) with their closest points on the reference, solves a 6x6 system for the small turn and
move that shrinks the distances along the reference face normals, and composes it onto the transform.
It stops when a round improves the RMS by less than `tolerance` of itself, or after `max_iterations`.
Pairs farther than `max_pair_distance` are ignored, for scans that only partly overlap. Returns the
transform (`p' = R p + t`, mm), RMS before and at the start of the last round, rounds, pairs, converged.
Limits: it finds the nearest fit, so the scan must already be roughly in place (pass a `start`); a flat
or round scan can slide along the reference and is not constrained by ICP alone; the pairing sign near
edges follows the closest triangle, so residuals beside an edge can differ by a percent or so between
frames. Checks: an L-shaped part moved by 2.9° and about 0.8 mm is moved back within 2e-3 mm; an aligned
scan stays put; moving both meshes together gives the same residuals within 2%.


## Datum frame (2026-11-07)

`align::datum_frame(primary, secondary, tertiary)` takes three fitted planes as (point, unit normal) and
returns the transform from the part's coordinates onto its datum frame: origin where the three planes
meet, Z the primary normal, X the secondary normal with its Z part removed, Y = Z × X. The tertiary plane
only fixes the origin. Refused: a zero normal, primary and secondary within 10 degrees of parallel, and
three planes that do not meet at one point (determinant of the normals below sin² 10°). Normals set the
axis signs, so give them the same side on every part of a batch. Checks: planes taken from three faces of
a part moved by an arbitrary turn and move return every vertex to its original place within 1e-6; each
refusal has a case, including 5 degrees off parallel with a good third plane.
Not done: plane-line-point datums and CLI/MCP exposure.
