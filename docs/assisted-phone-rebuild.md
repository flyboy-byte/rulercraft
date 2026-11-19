# Measurement-assisted rebuild from a phone capture

Verification and proposed scope, 2026-11-19.

A maker who has a rough phone capture, calipers or a tape measure can use the capture for the visible
shape and feature locations, supply dimensions where the capture is weak, and get an editable first
draft in SolidCraft. This is an optional route through the existing phone-scan and editable-CAD intent.
It shares the CLI and library geometry. MCP can guide the user through choosing reference points,
identifying features and entering measurements. The same inputs should work without an agent.

## What works today

I exercised the binaries from the scan-volume branch at `fa80893` (based on `df26fd6`) and SolidCraft
CLI 0.45.0-rc.4. The inputs were generated with known dimensions.

| Capability | Observed result |
| --- | --- |
| CLI point-cloud input | XYZ and vertex-only ASCII PLY both fit the same biased cylindrical wall to 8.704 mm. `measure` returned exactly 6 mm between two synthetic planes in both formats. |
| Hand diameter in a cylinder fit | Supplying `--known-diameter 8` returned `assumed_diameter: 8` alongside the free 8.704 mm diameter. Half-scale input still fitted 4.352 mm with the same assumption. A zero assumption was refused. |
| MCP | The running stdio server refused both cloud files at `mesh.open`. It opened the known hole-block STL and fitted a selected 3 mm hole with `known_diameter: 3`. The tool list contains no measurement-anchored rebuild command. |
| Automatic editable rebuild | `rulercraft solidcraft` refused both cloud files because they lack triangles. `project create` also refused the XYZ. Existing mesh rebuilds can produce an editable SolidCraft design. |
| Evidence limits | A box selection missing the cylinder returned a refusal with zero points. A biased full wall with only 0.05 mm added noise had about 0.05 mm fit sigma while the diameter missed the physical reference by about 0.70 mm. A good residual can coexist with scale error or systematic capture bias. |

### Partial coverage and noise

The wall had a true center at (15, 12) mm and a reference diameter of 8 mm. The observed points had
a deliberate +0.35 mm radial bias, 240 samples per case, uniform angle over 90°, 180° or 360°, uniform
height over 10 mm, and independent Gaussian noise per coordinate at 0.05, 0.25 or 0.60 mm standard
deviation. Each combination used Python random seeds 7, 19 and 47, with draws per point in order:
angle, height, x noise, y noise, z noise. With clipping disabled, each of the 27 clouds was fitted
with no diameter assumption, with 8 mm, and with 10 mm. The free diameter, center and residual were
the same for all three runs on each cloud to the reported precision.

Across the 81 fits the free diameters ranged from 8.556 to 9.505 mm. In the three 90° cases at
0.60 mm noise the center estimate missed by 0.180 to 0.429 mm, measured as the XY distance from the
returned axis point to the known axis. This sweep isolates noise, coverage and bias. Phone capture
can additionally introduce occlusions, drift, softened edges, uneven density and holes in the data.

### Editable result assembled from the evidence

I generated a second, labelled partial cloud with an artificial 2× scale error, noisy samples from
one side of a rectangular plate, and a biased quarter-cylinder wall. A scratch script used a
60 mm hand reading and two corresponding fiducials 30 input units apart to rescale the points.
It asked the CLI for a plane and cylinder fit, then assembled a named-parameter plate and hole in
SolidCraft, using the supplied 10 mm thickness and 8 mm diameter. Width and hole center were held
out from the supplied values.

- The fitted width was 30.0013 mm and the center was (15.0057, 11.9870) mm against (30, 15, 12) mm.
  This used a supplied datum and preselected regions. The free wall fit still estimated 8.7923 mm.
- SolidCraft evaluated 17,498.1428286 mm³, matching the analytic volume for the emitted dimensions.
  Editing thickness to 12 mm gave 20,997.7713943 mm³. Changing the selected hole diameter from
  8 to 10 mm at the original thickness gave 17,215.3994898 mm³.
- Replacing the calibration reading with 66 mm made the held-out width 33.0015 mm and failed the check.
  The supplied thickness and diameter are inputs. Their agreement in CAD checks parameter wiring.

This establishes that the component pieces can produce an editable part with explicit assembly.
The current product lacks a straight-through point-cloud import, calibration, hand-measurement
binding and editable rebuild over MCP. Physical phone accuracy remains unverified.

## Smallest useful implementation

1. Add cloud-aware open and plane/cylinder fitting over MCP using the reader the CLI uses. A labelled selection should
   produce the same results from the CLI and MCP. Triangle-only commands should give an actionable refusal.
2. Add an explicit calibration operation using a pair of points and a measured distance. Record the original unit,
   reference points, measured value and resulting scale. Keep raw coordinates available. Reject coincident points
   and nonpositive or nonfinite lengths. Report disagreement with additional independent references.
3. Build one rectangular plate with a single through-hole from selected scan regions and named hand measurements.
   Store each parameter's source and its supporting fit or measurement. Show the free fit beside a user-entered
   value when they disagree. Ask for a location or dimension when the selected region gives insufficient evidence.
   Write editable parameters and report the values still uncertain. Use direct fits for this slice. General
   surface reconstruction remains useful for shapes outside this scope.

A synthetic acceptance run should start with a cloud in arbitrary scale and a known plate, give only a scale
reference, thickness and diameter, and hold out width and hole center. Exercise both correct and wrong anchors,
missing wall coverage, contradictory readings, and an edit in SolidCraft. Set recovery thresholds from the
fixture and intended first-draft use before running the test. Keep phone capture evaluation as a separate check
once a physical part and measurements are available.

## Where it belongs

- [INTENT.md](../INTENT.md) already covers phone scans, trustworthy dimensions and editable CAD.
- [v1](v1.md) can describe the assisted route under scan-to-editable CAD and agent use. A synthetic
  acceptance run gives engineering evidence for the route. S1 still needs a real part.
- [Scan sources](specs/scan-sources.md) owns supported phone exports and the calibration contract.
  `io.pointcloud` supplies the reader, and `scan.phone` tracks existing capture exports.
- [MCP](specs/mcp.md) owns cloud open, region selection and the agent-facing operations.
  [Mesh to CAD](specs/mesh-to-cad.md) owns how fitted and supplied dimensions become editable parameters
  and how their sources and disagreements are reported.
- [Project file](specs/project-file.md) is the place to specify durable evidence when this workflow needs to
  reopen. The current format stores mesh fit recipes.
