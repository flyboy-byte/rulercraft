# Measurement-assisted rebuild

The [next milestone and physical trial](../assisted-rebuild-next.md) describes the desktop
integration and comparison with the host's native workflow.

A rough capture supplies geometry that is visible. Calipers or tape supply scale and dimensions the
capture cannot establish. The first supported part is a rectangular plate with one perpendicular,
circular through or blind hole. See the [capture evidence](../assisted-phone-rebuild.md).

| Feature | Contract | Completion evidence |
| --- | --- | --- |
| scan.scale | Check the actual calibrated anchor after storing coordinates. Preserve the existing file-origin convention and report precision loss when stored coordinates cannot represent the scale. Carry the source unit, reference and optional independent check forward. | The translated four-point regression, an exact known span and a wrong-ratio run. |
| scan.picks | Return selected vertex coordinates in bounded pages alongside IDs and the current scale. Let the caller positively identify the supports for the reference, faces and bore. | Shuffled cloud, paginated and filtered picks, stable ID-to-coordinate mapping before and after calibration. |
| cad.assisted | Take selected scan evidence and explicit hand dimensions. Emit a SolidCraft document with editable named parameters, their sources and unresolved decisions. Require explicit through status, or measured blind depth. Missing or weak bore evidence requires a hand-supplied center or a request for more evidence. | Held-out width and center on an arbitrary-scale, rotated, noisy partial cloud. Exercise incorrect references, contradictory dimensions and absent bore samples. Evaluate and edit thickness in SolidCraft. |
| sc.assisted | Expose the same supported parameters and evidence through an agent and through the SolidCraft workflow. Carry the report with the result and verify against the host solid. | Known part rebuilt and edited through the normal entry points, with matching parameters and explicit uncertainty. |
| verify.phone | Exercise the workflow on an untouched phone export. Record file format, units, supplied references, held-out caliper values and which visible features supported the result. | Import, rebuild and edit inside SolidCraft. Compare against declared physical tolerances and document any refused or unresolved features. Keep capture local. |

Uniform scale does not correct local drift. Fit residuals describe the observed samples. Hole depth and
other hidden geometry require an independent observation or an explicit decision. Retain both hand and
fitted values when available. Use disagreement and support quality to decide when a user needs to intervene.

Keep source import, feature inference, typed dimensions and SolidCraft emission separate. Use the exact
mesh fixtures to protect the existing importer when extracting shared CAD emission.

## Picked plate draft over MCP

`assist.plate` takes three top corner vertex IDs ordered as origin, length corner and width corner.
In the reported frame the length runs along the first edge. Width is the component of the second
edge perpendicular to the first. The edges must be within 10° of square. The result reports both
edge directions and the observed angle. Length and width come from the picks. The caller supplies
thickness, diameter and center offsets in mm from the first corner along those axes. The center is
an explicit caller input, with an optional note identifying a hand measurement or prior fit.
Supply exactly one depth choice, `{"through":true}` or `{"blind_depth_mm":4}`.
Reject nonpositive dimensions, holes touching or crossing an edge, and blind depth at or beyond
the bottom. Return `design` for SolidCraft and `evidence` describing every dimension. The design
uses a local XY plane. The report locates that plane's top in the cloud.

A controlled 2026-11-21 check used `capture` from `tools/dev/check_assisted_cloud.py` with seeds
7, 11 and 17, turns of 0° and 31°, noise of 0.05 and 0.30 mm, and no bore wall. For each known top
corner, it picked the nearest actual vertex to the generator's expected corner. The first two picks
and a 60 mm hand reading set the scale. The draft used a supplied center and through-hole decision.
Across 12 clouds, the held-out 30 mm width came back as 29.324–29.856 mm, with a median of 29.559 mm.
The picks underestimated width on every trial. Use this to compare a future face-fit method with
individual corner picks. This check did not estimate the hole center.

For a fit-backed center, supply `bore_selection` (standard MCP filters) and
`max_diameter_difference_mm` instead of `center_mm`. The selected wall must support a cylinder
covering at least 180° whose axis is within 10° of perpendicular to the top. The freely fitted
diameter must agree with the hand reading within the caller's stated positive allowance. The
intersection of the fitted axis with the top sets the center. The response carries the free fit
and the disagreement. If the fit or a guard fails, select more wall or supply an explicit center.

A 36-cloud check used the same seeds, turns and noise as the picked plate run, with 0°, 90° or 360°
of actual wall. It used a 5.4 mm spherical selection around the generator's approximate bore pick
and a 0.5 mm diameter allowance. The exact design center was held out. All 12 absent-wall trials
were refused. All 12 full-wall trials produced a center, with 0.101–0.319 mm error in the picked
frame. At 90° all six low-noise trials were refused, while all six higher-noise trials passed,
with 0.103–0.378 mm center error. Selected top or noisy points can inflate the fitted coverage
statistic. The guard needs more evidence before it can establish how much true bore wall was seen.

### Width from selected side faces

Optionally provide `width_faces` with `origin_side` and `opposite_side` selection filters and a positive
`max_sigma_mm` per face. Fit a plane to each selected side. Both normals must be within 10° of the
picked width axis and of one another, ignoring sign. Measure between the plane intersections along
the width axis through the fitted patches’ midpoint, projected onto the plane through the first
picked corner perpendicular to the width axis. This limits extrapolation away from the measured
points. Reject reversed, coincident or unresolved planes.
The width must exceed twice the sum of the fit residuals (sigma) projected along the measurement axis.
Each selection is limited to one million points.

The fitted origin-side intersection shifts the design origin and the reported frame along the width
axis. A caller-entered `center_mm` is still relative to the first picked corner. Its Y offset is
translated into the fitted frame. A fit-backed center is determined directly in the fitted frame.
Return both plane fits, their offsets and the width originally estimated from the picks. Omitting
`width_faces` uses the picked-corner width.

On the same 12 noisy, absent-wall captures, two 3 mm-radius selections around the approximate side
picks reduced the maximum width error from 0.676 to 0.396 mm. The fitted widths were
29.604–30.236 mm, with a median of 30.122 mm. The per-face RMS ceiling was twice the injected
per-coordinate noise. Measuring at the picked corner instead yielded 28.899–31.519 mm.

Adding those two side selections to the 36-cloud bore sweep still refused all 12 absent-wall cases.
All 12 full-wall cases recovered centers within 0.311 mm in the fitted frame. At 90°, the six
low-noise selections refused and the six higher-noise selections produced up to 0.376 mm center
error. One 31° full-wall case with 0.30 mm injected noise produced a 30.006 mm width. SolidCraft
evaluated its emitted part at 14,000.538 mm³ and at 21,000.807 mm³ after a thickness edit from 8
to 12 mm. Both volumes agreed with the analytic volume of the emitted dimensions within 1e-6 mm³.
