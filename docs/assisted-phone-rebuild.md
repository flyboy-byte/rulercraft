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
MCP now imports and calibrates the cloud. Binding hand measurements to editable SolidCraft dimensions
still requires explicit assembly. Physical phone accuracy remains unverified.

## Whole-part verification

A second probe used the CLI built on this documentation branch from engine revision `df26fd6`.
It mixed and shuffled the top, exterior walls and visible bore wall of a 60 × 30 × 8 mm plate in
one XYZ file. Its bore is 8 mm across at (15, 12). The underside and lower half of the bore are
unobserved. Each trial used one of three noise seeds, two poses, two noise amplitudes, three bore
coverage angles, and one of three hand readings for a 60 mm reference. The 108 combinations reuse
the same noise realization across poses and reference readings. They are paired comparisons.

The harness supplied two approximate scale-reference picks (0.15 mm standard deviation per coordinate)
and three approximate region picks (0.35 mm). It applied a uniform scale calculated from the hand
reading and reference picks. The CLI saw the full cloud with spherical selections around the picks.
It fitted two side planes and a cylinder with the supplied 8 mm diameter assumption. Width below is
a harness diagnostic projecting the two plane centers onto the first fitted normal. Bore diameter is
the free fit. Axis miss measures the distance between the true hole midpoint and the estimated axis
in physical millimetres relative to the first reference pick. The generator's true pose was used
for scoring. The case assumes a user or agent can identify the approximate regions.

With the correct 60 mm hand reading, six trials per row gave the following ranges.

| Visible bore arc | XYZ noise per coordinate | Returned bore diameter | Axis miss | Axis tilt |
| --- | --- | --- | --- | --- |
| 360° | 0.15 mm | 7.954–8.012 mm | 0.111–0.181 mm | 0.28–1.33° |
| 360° | 0.60 mm | 7.700–7.825 mm | 0.113–0.182 mm | 1.03–3.46° |
| 90° | 0.15 mm | 7.751–7.855 mm | 0.111–0.266 mm | 1.51–2.82° |
| 90° | 0.60 mm | 6.930–7.397 mm | 0.657–0.793 mm | 17.41–89.72° |
| 0° | 0.15 mm | All six refused | — | — |
| 0° | 0.60 mm | Four fits from nearby surface samples, 3.506–27.996 mm. Two refused. | 3.030–10.674 mm for returned fits | 83.91–84.41° |

For the full-arc clouds the 30 mm width estimate spanned 29.489–30.160 mm with a 60 mm reading,
30.003–30.662 mm with 61 mm, and 32.662–33.219 mm with 66 mm. A second independent length can
expose a scale error. A supplied hole diameter by itself cannot determine its center, direction or depth.

### Two different parts explain every visible point

SolidCraft CLI 0.45.0-rc.4 built two plates differing only in the bottom of the hole. One hole went
through. The other ended 3 mm above the base. The host measured volumes of 13,997.876 and
14,148.673 mm³, agreeing with the two analytic solids. Its exact-body projection reported zero maximum
distance for **the same 7,061 noiseless visible samples** on both. Adding one point on the hidden wall
at (19, 12, 1) gave 0 mm on the through part and 1 mm on the blind part. A measurement of depth or
an observation of the underside resolves an ambiguity that more samples on the visible surfaces
cannot resolve. This is also why the assisted part needs to record the source of a depth decision.

### Reproducing the checks

From the repository root, build the CLI and run the probe. It prints a JSON record per trial, including
refusals. Supplying the SolidCraft executable adds four records for the hidden-floor check and asserts
the expected volumes, measured point counts and distances.

```sh
cargo build --locked -p rulercraft-cli
python3 tools/dev/check_assisted_cloud.py target/debug/rulercraft --solidcraft solidcraft-cli
```

These trials verify the fit and CAD components under controlled conditions. They do not test automatic
feature discovery, unassisted point selection, nonuniform scale drift, a physical phone export, or a
complete MCP-only reconstruction.

## Measuring scale over MCP (2026-11-21)

A separate run opened the raw, shuffled XYZ output of `capture` in the MCP server. It used seeds
7, 11 and 17, rotations of 0° and 31°, noise of 0.05 and 0.30 mm, and 90° of visible bore.
For each simulated corner pick the probe chose the nearest actual cloud vertex. The caller supplied
a length and the two vertex IDs. Two local plane fits at opposite walls provided an independent
width estimate by projection onto the first plane normal. The generator supplied approximate pick
locations. Identifying the right corner or wall unaided is still untested.

| Hand reading for the 60 mm span | Trials | Recovered width for a 30 mm plate (min / median / max) |
| --- | ---: | --- |
| 60 mm | 12 | 30.042 / 30.138 / 30.234 mm |
| 66 mm | 12 | 32.998 / 33.176 / 33.261 mm |

## Smallest useful implementation

1. Implemented in #134: cloud-aware open and plane/cylinder fitting over MCP using the reader the CLI uses. A labelled selection should
   produce the same results from the CLI and MCP. Triangle-only commands should give an actionable refusal.
2. Implemented for MCP point clouds in #135: explicit calibration using a pair of points and a measured distance. Record the original unit,
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
