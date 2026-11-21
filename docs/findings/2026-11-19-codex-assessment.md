# Codex assessment — 2026-11-19

Reviewed `main` at `a556825` (merge of #125). The live plan gave 63.4% weighted progress over 74 features and showed #126 claiming `fit.shapes`. The generated status shows 68%, so use `python3 tools/plan.py progress` for weighted progress. `python3 tools/plan.py needs` listed 13 ready features, 11 started items with work left and no open findings at that revision.

## Baseline

Rust 1.100.0 passed `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (277 passed, five ignored). `python3 tools/plan.py check` passed. The reported PLY memory failure did not recur in this run.

## Scan to SolidCraft

- `solidcraft_design.rs` checks dimensions, feature kinds and positions, coordinate recovery for moved and turned CAD meshes, and a refusal for a lone sheet. Its thickness test checks the emitted parameter expressions. These are engine assertions over design and report JSON. The scan tests check selected dimensions and report entries at two sampling resolutions, with one pose and one noise amplitude. Their upper bound for reported p95 is 0.1 mm. They leave independent CAD evaluation, physical scans, incomplete scans and a sweep of poses open.
- With SolidCraft CLI 0.45.0-rc.4, `plugin/solidcraft/build.sh` built the component and passed all 14 host plugin checks. I separately ran `rulercraft solidcraft fixtures/known/<part>.stl --out /tmp/rebuilt.sc` and `solidcraft-cli eval /tmp/rebuilt.sc` for the ten fixtures with original design files. All ten agreed with their original SolidCraft volumes within 0.000000001 mm³. The hole block separately measured 17,230.309799870498 mm³, agreeing with a 60 × 30 × 10 box minus its three cylindrical bores. Changing the bracket's rebuilt `thickness` expression from `8 mm` to `9 mm` changed the host volume from 31,282.21237390821 to 35,253.9380400259 mm³. The difference agrees with the 4,000 mm² footprint less the Ø6 through hole, times 1 mm. The wider claim of 0.25% for every synthetic scan remains unverified here.
- Both scan tests fail at another pose with the same mesh resolution and noise. In a scratch checkout, replace `23f64.to_radians()` with `47f64.to_radians()` in `crates/rulercraft-mesh/tests/scan_like.rs` and run `cargo test -p rulercraft-mesh --test scan_like -- --nocapture`. The bracket's pocket, which is square in the source SDF, is dropped as `pocket_rotated` at 5.011 mm. The detailed plate reports two fillet segments (11.031 and 18.729 mm long, radii 2.045 and 2.018 mm) for one designed edge. The duplicate report warrants checking the emitted geometry in the host before diagnosing a CAD kernel failure.
- An exact 50 × 30 × 8 mm plate with a 5 × 2 mm notch, through the thickness and open on the long edge, rebuilds as a complete box. To reproduce, extrude the polygon `(0,0), (20,0), (20,2), (25,2), (25,0), (50,0), (50,30), (0,30)` by 8 mm, export STL and run the rebuild and host eval commands above. For an equivalent triangulation, split the plate at x=20 and 25 and y=2, omit the notch cell, and triangulate the exterior quads of the remaining cells. The rebuilt volume is 12,000 mm³ against the analytic 11,920 mm³. The CLI gives no loss item and returns p95 below 0.000001 mm for that triangulation. Both the outline classifier and the later coverage warning permit the missing 0.67% of the base.
- `details.check` evaluates sampled scan vertices against an in-memory signed-distance approximation assembled before design serialization. It is one-way, depends on vertex density and can miss extra CAD geometry. `MAX_CHECK_POINTS` is nominally 200,000, but floor division chooses stride one for 399,999 vertices. `details.volume_check` asks the host for volume and compares it to a signed tetrahedral sum whose absolute value has a physical interpretation for a closed, consistently oriented mesh. Equal volumes can hide a displaced hole or swapped features. Open scans still get a numeric comparison and a note.
- `design` starts at line 765 and spans about 1,700 lines. Extract the graph, segmentation, part frame and tolerance choices first (765–1038), then isolate boundary tracing and snapping (1039–1256). The base and opposite face are the largest qualifying regions, so fragmented top faces or large parallel intermediate levels can set the wrong thickness. The boundary walk chooses unused edges by vertex index at a junction and discards open chains. `Level::aligned` uses the pocket floor's smallest enclosing rectangle before it checks the wall evidence used for snapping. The 47° test is a concrete failure at that seam.
- Next separate feature recognition and ownership of consumed triangles (1257–2138) from distance checking and the SolidCraft lowering (2139 onward). Use typed holes, pads and pockets across these stages. Today internal JSON is read back by key with zero defaults, so a producer/consumer mismatch can silently turn a dimension into zero. Keep the numerical behavior covered by the existing exact fixtures while moving it.

## Point clouds

The tests establish known fits for a synthetic XYZ sphere, PTS cylinder in centimetres and ASC plane, and they check vertex-only PLY and an ordinary PLY mesh read as a cloud. The CLI test also measures planes 6 mm apart and checks the selection and segmentation refusals. The malformed-text cases cover invalid UTF-8, nonfinite coordinates, a vertex limit and bad rows after the first point.

At the reviewed revision, the first non-comment line is skipped on any coordinate parse error. This can silently delete a measurement. The following command succeeds with three points, although the first row starts with two coordinates and has a malformed third coordinate.

```sh
printf '10 20 bad\n0 0 5\n10 0 5\n0 10 5\n' >/tmp/rulercraft-bad-first.xyz
cargo run -q -p rulercraft-cli -- fit plane /tmp/rulercraft-bad-first.xyz
```

A scratch library test requiring `read_cloud(...).unwrap_err()` failed with the three remaining positions. New tests should distinguish an actual header from a mixed coordinate row at the start, and retain the existing known fits.

## Fuzzing

A 10-second release run with seed 47 passed 2,707,256 reader mutations and 10,484,975 fit requests. Reproduce with `RULERCRAFT_FUZZ_SECONDS=10 RULERCRAFT_FUZZ_SEED=47 cargo test --release -p rulercraft-mesh --test fuzz -- --ignored --nocapture`. The deterministic run mutates small, valid seeds and checks finite positions, valid returned triangle indices and output counts for successful reader calls. It catches unwinds and exercises plane, sphere and cylinder requests on one octahedron. It cannot establish that malformed records are rejected: the cloud from the reproduction above satisfies every reader success assertion. A concrete semantic regression belongs beside the format tests. A per-input elapsed-time assertion runs after the call returns, so a stuck case requires an outside timeout. The short run's minimum is a total count across seeds and gives no minimum coverage per format or successful fit kind.

The fit finiteness predicate also accepts unrelated nulls whenever a diameter field matches its exception. For a direct reproduction, feed this serialized value into the condition in `fuzz_commands`: `{"free_diameter":null,"center":[null,0,0]}`. It passes despite the invalid center. Check the fitted fields as numbers, with absence permitted only on the individual optional field.

## Next three changes

1. Refuse mixed numeric and nonnumeric first point rows. It is a small change with a reproducible silent measurement failure and independent known fits to protect the accepted formats.
2. Preserve small, well-resolved notches in exact outlines. Use the notch volume as a known answer in SolidCraft, then change thickness and check the volume again. The current volume and loss report can appear to disagree about whether the part needs correction.
3. Add the 47° bracket as a known-answer regression and use the pocket walls to corroborate floor orientation. Check actual SolidCraft volume and a parameter edit. This protects a common editable feature without widening the angular tolerance.
