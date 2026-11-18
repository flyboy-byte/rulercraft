# Signed deviation on a zero-area reference triangle (2026-11-07)

**Reproduced.** At main `dc2aeae`, a reference OBJ containing the vertices `(0,0,0)`, `(1,0,0)`,
`(2,0,0)` and face `1 2 3` is accepted. A measured OBJ with those same x coordinates at z = -1,
and another at z = +1, each produce three signed distances of +1 mm via `rulercraft deviation`.
Both results report `max_outside: 1` and `max_inside: 0`.

**Why it matters.** The reference face is collinear, so its normal is zero and its winding cannot
establish an inside or outside. The unsigned distance is 1 mm, but assigning a positive side and
counting it as outside presents an unsupported sign as a measurement. This can also affect patches.
The import contract only drops triangles with repeated merged vertices, so a zero-area face with
three distinct vertices can reach comparison.

**Cause.** Deviation takes the positive branch whenever the dot product with the closest face normal
is not negative, including a zero normal. The deviation spec describes sign as following that normal;
it does not currently define this case.

**Not decided.** Possible remedies include refusing an undefined closest-face sign or excluding
zero-area reference faces before building the index. They differ for mixed meshes and ties, so the
reproduction does not choose between them. Any fix should cover both sides of a collinear reference,
a mixed valid/degenerate reference, and the behavior when every reference triangle is zero-area.

**Reply 2026-11-12 (Claude): Closed.** Zero-area reference triangles are now left out of the reference index
(`deviation::Reference::new`, used by deviation, patches, ICP and the recipe check); a reference whose
triangles all have zero area is refused with a plain message. The three cases asked for are tested in
`tests/deviation.rs`: both sides of an all-collinear reference refuse; a collinear triangle along a valid
face's edge no longer sets the sign (below the face was +1, now -1). Zero area means an exact zero cross
product of the f32 corners in f64; near-zero slivers keep their (roundoff) normal and are not addressed here.
