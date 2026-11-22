# Assisted rebuild in SolidCraft

Status on 2026-11-22. Build this first as a RulerCraft plugin workflow. Keep an upstream SolidCraft
change as an option once a concrete host limitation is demonstrated. This is a near-term priority
within [RulerCraft's intent](../INTENT.md).

## What the plugin should save the maker

A maker has an imperfect phone capture and a part in hand. Use a measured span to calibrate the
capture, use visible geometry to locate features, and take critical dimensions from calipers or tape.
Save an editable SolidCraft part and enough evidence to explain each decision. Start with a
rectangular plate and one round hole. Through status or blind depth requires an explicit choice
or a physical observation.

## What SolidCraft already covers

SolidCraft 0.46 can open E57 scans, measure selected points and place datums. Its Convert Mesh
operation makes an editable solid from suitable closed meshes. The 0.46.2 tests cover cylindrical
face recovery on a bored part. A separate local check on 0.45.0-rc.4 converted the known hole block
and bracket. Each gained a Convert Mesh feature and no named user parameters. This check did not
compare the quality or time required to edit a physical scan. [SolidCraft 0.46 release notes](https://github.com/tangency-labs/solidcraft/releases/tag/v0.46.0)
· [SolidCraft 0.46.2 conversion tests](https://github.com/tangency-labs/solidcraft/blob/v0.46.2/crates/engine/src/tests/mesh_convert.rs)

RulerCraft's existing plate importer infers named features and reports losses. Its assisted MCP
path adds calibration, hand dimensions, and fit evidence. The value to prove is less remodeling
when a capture is imperfect. Native measurement, general closed-mesh conversion and E57 import can
be used wherever they already serve the workflow. Keep dedicated readers, generic faceted conversion,
and branch reconstruction in the roadmap. Revisit their priority when a user workflow demonstrates
a gap. [Support by interface](capabilities.md)

## First engineering milestone

Claim `sc.assisted` and start with the installed plugin on a current supported SolidCraft host.

1. Check the host contract for passing selected scan evidence to RulerCraft and retaining dimension
   evidence alongside an editable design. Test these two operations independently and record the
   host version, command or interface, and observed outcome. The prior mesh size limit is a lead
   to check on the current host.
2. Reuse the calibration and assisted plate behavior exercised by RulerCraft MCP. Verify the same
   explicit picks and readings from the plugin and MCP on one generated capture. Keep the draft's
   plate parameters, source record and limits consistent.
3. Open the result through the desktop workflow, save and reopen it, then edit thickness and
   diameter. Verify the features move or resize as intended and preserve the source record.
   Include an underconstrained case that asks for the missing dimension or decision.
4. Run a bounded synthetic check with a biased scale, noisy walls, a rotated part and a missing
   bore. Keep the known width and center out of the input. Set tolerances before running from the
   fixture's sampling and the intended first-draft accuracy. Record fit and refusal outcomes.
   The existing [contract and trials](specs/assisted-rebuild.md) are the starting point.

This work can begin while the physical capture is pending. If an interface blocks a usable
plugin workflow, isolate the smallest missing operation and keep an exact reproduction. Consider
an upstream PR once the workflow demonstrates a benefit and the blocker is reproducible. Logan
decides whether to contact maintainers.

## First physical phone trial

### What Logan can send

- A rough phone export of a real plate or bracket with one round hole, saved as it came out.
- Measured overall length, width, thickness and hole diameter, with units. Say whether the hole is
  through. For a blind hole, include depth if you can measure it.
- The plugin's import report or a screenshot of its response after one attempt, if convenient.

The minimum useful start is the export and one measured length. A scan with insufficient evidence for
one of the features is a valid result to record. Keep supplied files and measurements in
`fixtures/private/` or `local/` as appropriate. The repo gets only original synthetic fixtures and
non-identifying results approved for sharing.

### What the next session does

- Record the unmodified export format and units, tool versions, approximate anchor positions, and
  measurement repeatability if supplied. Supply length for scale, thickness and diameter as hand
  dimensions. Reserve width for verification. Reserve hole center as a second check if it was
  independently measured.
- Try the host's native workflow on the same capture. Record what opens, what measures, what
  converts, and the editing work required. Give both workflows access to the same hand readings.
  Use the same output targets and note any refusal or additional hand modeling.
- Run the assisted rebuild, preserve its evidence, and score the held-out dimensions independently.
  Check saved-project reopen, a thickness edit and a hole-diameter edit. A supplied dimension is
  evidence of parameter wiring. A held-out dimension measures recovery.
- Report raw results, meaningful uncertainty, any missing geometry, and whether it saved remodeling
  time. Classify the next gap by what the trial actually exposed: scale, feature evidence,
  feature generation or desktop integration. A trustworthy refusal is a valid outcome.

One physical comparison can justify continuing this workflow. Broader shape coverage needs
additional measured parts. The current release and implementation status are in [HANDOFF.md](../HANDOFF.md).
