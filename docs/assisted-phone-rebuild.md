# Measurement-assisted rebuild from a phone capture

Controlled trials started 2026-11-19. Implementation status updated 2026-11-21.

A maker who has a rough phone capture, calipers or a tape measure can use the capture for the visible
shape and feature locations, supply dimensions where the capture is weak, and get an editable first
draft in SolidCraft. This is an optional route through the existing phone-scan and editable-CAD intent.
It shares the CLI and library geometry. MCP can guide the user through choosing reference points,
identifying features and entering measurements. The same inputs should work without an agent.

## Baseline before the assisted MCP implementation (2026-11-19)

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
The later MCP implementation binds hand readings and selected fits to editable dimensions.
Physical phone accuracy remains unverified.

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

## Current implementation (2026-11-21)

- #134–#139 added MCP cloud import, measured calibration, rejection when stored geometry loses the
  requested span, and bounded selection pages with coordinates.
- #140–#142 added `assist.plate`. Use a hand thickness, hand diameter and an explicit depth choice.
  Width comes from corner picks or opposing side faces. Supply a center or fit it from selected
  bore samples that meet the coverage, axis and diameter checks. The result carries the
  editable design, dimension evidence and scan frame.
- Synthetic trials held out width and center and evaluated a thickness edit in SolidCraft. All
  12 absent-wall cases refused in the combined sweep. Some noisy quarter-wall selections passed,
  so fitted coverage is still an imperfect proxy for observed wall. See the
  [assisted rebuild contract and results](specs/assisted-rebuild.md).

The next roadmap items are `sc.assisted` for native SolidCraft integration and `verify.phone` for a physical
scan with held-out caliper measurements. Automatic identification of reference points and regions
also remains unverified.

## Where it belongs

- [INTENT.md](../INTENT.md) already covers phone scans, trustworthy dimensions and editable CAD.
- [v1](v1.md) tracks the assisted route alongside scan-to-editable CAD and agent use. S1 still needs a real part.
- [Scan sources](specs/scan-sources.md) owns supported phone exports and the calibration contract.
  `io.pointcloud` supplies the reader, and `scan.phone` tracks existing capture exports.
- [MCP](specs/mcp.md) owns cloud open, region selection and the agent-facing operations.
  [Mesh to CAD](specs/mesh-to-cad.md) owns how fitted and supplied dimensions become editable parameters
  and how their sources and disagreements are reported.
- [Project file](specs/project-file.md) is the place to specify durable evidence when this workflow needs to
  reopen. The current format stores mesh fit recipes.
