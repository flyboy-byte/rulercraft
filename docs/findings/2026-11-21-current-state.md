# RulerCraft readiness on 2026-11-21

Reviewed main at `0f44469`. A maker can use an agent to calibrate a rough point cloud and fit selected features. A plate-like triangle mesh can become an editable SolidCraft part. Producing that part directly from selected cloud features plus hand dimensions still takes external assembly.

## What ran

- A clean build passed `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (288 passed, five ignored). The short reader and fit fuzzer ran in this gate. The long fuzz run was outside this assessment.
- `python3 tools/plan.py check`, `progress` and `needs` ran. Python unit tests passed with one skip. Hosted CI passed on the reviewed merge commit. The plan reports 65.1% weighted completion across 74 features, including 35.0% for scanner inputs and 47.9% for v1 inside SolidCraft. These percentages measure roadmap status. The v1 acceptance criterion requires a user completing the four jobs on their own parts in SolidCraft.
- Both synthetic scan rebuild tests passed at the default 47° pose and again at 61° using the compiled test binary with `RULERCRAFT_SCAN_TURN=61`.
- Rebuilt the plugin with `plugin/solidcraft/build.sh` and ran SolidCraft CLI 0.45.0-rc.4. All 14 host checks passed. The host warned that the requested 1,024 MiB exceeds the ordinary 128 MiB ceiling. GitHub listed zero releases at review time. Desktop installation and interactive inspection were outside this run.
- Rebuilt `fixtures/known/hole-block.stl` with `rulercraft solidcraft` and evaluated the resulting design with `solidcraft-cli eval`. Volume was 17,230.309799870498 mm³, consistent with the designed block and three bores.

## What could change a maker's result

**The cloud-to-CAD handoff is still manual.** The CLI rebuild refused a four-point XYZ because it needs triangles. MCP exposes cloud selection, fitting and measurements, while its reverse command requires triangles. A hand diameter can accompany a free cylinder fit. The product still needs a way to bind selected planes, a hole and hand dimensions to one editable part, identifying the source of each parameter.

**A plausible fit can describe a hole the scan never saw.** Reran `python3 tools/dev/check_assisted_cloud.py target/debug/rulercraft --solidcraft solidcraft-cli` (108 CLI trials plus four host comparisons). With the correct scale, 0.60 mm noise and zero visible bore wall, four of six trials returned a cylinder from nearby samples. Free diameters were 3.506 or 27.996 mm on an 8 mm design. With a 90° visible arc and the same noise, axis error reached 89.72°. The harness supplied approximate picks. A fitted cylinder alone is insufficient evidence to create a hole.

**Depth can be genuinely unknowable.** The same 7,061 visible points had zero maximum deviation from both a blind-hole solid and a through-hole solid. Their host volumes differed by 150.796 mm³. One additional point on the hidden wall distinguished them. A hand depth, underside observation or explicit through-hole choice belongs in the editable result.

**Measured scale works locally but can lose its own reference at large coordinates.** Repeated the 12 near-origin, noisy plate cases using MCP calibration and actual cloud vertices. For the 30 mm held-out width the range was 30.042–30.234 mm. A deliberately high 66 mm reading for the 60 mm anchor gave 32.998–33.261 mm. These are controlled inputs with supplied region picks. Physical capture accuracy is open.

A separate four-point check isolates an origin-dependent precision loss. Save the following as `span.xyz`:

```text
10000000 0 0
10000003 0 0
10000000 4 0
10000003 4 0
```

Call MCP `mesh.open` with the file path and this calibration:

```json
{"vertices":[0,1],"distance_mm":6.6,"check":{"vertices":[2,3],"distance_mm":6.6}}
```

Import succeeds. The original 3 mm span is exactly representable. The reported factor is 2.2, yet both the calibrated X extent and the check span are 6.0 mm. Subtract 10,000,000 from every input X and the measured check becomes 6.599999905 mm. At a 1,000,000 mm offset it is 6.5 mm. Calibration multiplies absolute coordinates, stores them in f32, then checks only whether the original anchor still has nonzero length. Large file origins therefore matter even when the part itself is small. The optional check exposed the error.

## Priorities

1. Check that the stored calibrated anchor reproduces its reading. Cover translated copies of the same known geometry. Preserve a local frame and its transform, or report a precision limit when the requested reference cannot be represented.
2. Deliver one assisted editable plate with one hole. Require selected support or an explicit hand decision for each CAD parameter. Test missing bore coverage, unknown depth, a held-out width and hole center, then edit thickness in SolidCraft and check the resulting volume. This closes the gap the maker actually encounters without requiring general surface reconstruction.
3. Establish the physical and desktop acceptance case when a measured phone capture becomes available. Preserve the untouched export, record its format and at least one held-out caliper dimension, then import, rebuild and edit the part inside SolidCraft. Until then, keep synthetic accuracy and end-user readiness distinct.

For maintenance, `crates/rulercraft-mesh/src/solidcraft.rs` is 2,992 lines. Its rebuild mixes recognition, internal JSON lookup, deviation checking and CAD emission. Several missing numeric fields default to zero. When adding the assisted path, use typed dimensions with their sources and extract CAD emission behind the existing known-design tests.
