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
