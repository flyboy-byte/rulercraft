# Measurement-assisted rebuild

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
