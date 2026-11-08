# Product scope: a tool people choose

Status: *Scoping (2026-11-07)*. Features: the R7 group (`fit.shapes`, `measure.constructs`, `measure.gdt`,
`align.auto`, `section.cut`, `mesh.clean`, `engine.wasm`, `cad.step`, `gui.*`). Serves INTENT.md.

Established browser-based scan-inspection apps already do a lot, free, with nothing to install. To be chosen
over them we must do the everyday job as well, and beat them on the things listed under "Where we win". This
page lists the capabilities in our own words (behaviour, not anyone's wording or layout), what we build in
what order, and how each is checked. We do not name the other apps here (D-001).

## The everyday job (table stakes)

A person opens a scan and does some of these. Each needs to work without reading a manual.

| # | Capability | Have today | Gap |
| --- | --- | --- | --- |
| T1 | Fit sphere, cylinder, plane, cone, torus, circle, with σ and form error; mark by click-to-grow, brush, window or lasso | plane, sphere, cylinder; click-paint brush; region click | cone, torus, circle; window and lasso; click-to-grow |
| T2 | Build points, lines, planes from fits; dimensions (distance, angle) between them; each with nominal ± tolerance and pass/fail | relations between two fits | constructions, dimension list, tolerances, pass/fail |
| T3 | Form and position checks: flatness, parallelism, perpendicularity, coaxiality, and similar | flatness (form error) | the rest, as named checks |
| T4 | Align a part: guided three-plane datum, automatic alignment from its own faces, symmetry plane | datum frame, ICP, symmetry test (engine) | guided UI, auto-align from faces, applying the transform |
| T5 | Deviation from the design: signed colour map, hover and pin readings, regions of extra or missing material; nominal from a mesh or from CAD | deviation numbers and patches (engine, MCP) | colour map and readings in the app, volume-error regions, CAD nominal |
| T6 | Wall thickness map | thickness samples (engine, MCP) | map in the app |
| T7 | Sections: cut across a feature and measure the cut in 2D; the part's widest outline (for bins, cutouts, pockets) | none | section and outline in engine and app |
| T8 | Take results out: CAD (STEP of the measured features), measurement report (CSV), pictures | recipe JSON | STEP via SolidCraft, report export, screenshot |
| T9 | Mesh clean-up: fix holes and bad triangles, reduce triangle count | none | clean and reduce in engine and app |
| T10 | No install, smooth on big scans, save and resume, undo | local app, small meshes, no save in page, no undo in page | browser build, performance, projects, undo |

## Where we win

W1 permissive licence; W2 engine first, so the same checks run in the app, a script, CI or an agent;
W3 numbers backed by known-answer tests, with conventions and tolerances shown; W4 the mesh-to-CAD loop
closes through SolidCraft; W5 design-intent help (patterns, symmetry, round-number guesses with evidence);
W6 local and free of strings. These are what we put forward; the table stakes are what let people stay.

## Order of work (slices)

Each slice ends in something Logan can do and check. Slices are independent where the dependencies allow.

1. **A real first measurement** (T2, T3, T10 part): in the app, mark, fit, build constructions, add dimensions with
   tolerances and pass/fail, undo, save and reopen the project, export a CSV report. Exit: from opening a file to
   a saved dimension report on the known test parts in under a minute, numbers equal the CLI's. (`measure.constructs`,
   `measure.gdt`, `gui.dimensions`, `gui.projects`, `gui.report`)
2. **Deviation** (T5): load a nominal mesh, auto-align, colour map, hover and pin readings, extra/missing regions.
   Exit: a scan of a known part gives a map whose numbers match the engine's within 1e-6 and whose picture shows a
   deliberately added bump. (`gui.deviation`, `gui.thickness`)
3. **Alignment** (T4): guided three-plane datum, auto-align from faces, symmetry plane, in the app.
   Exit: a rotated and moved copy of a part aligns back within 0.01 mm. (`align.auto`)
4. **Sections** (T7): section cut and outline in engine and app. Exit: sections of known solids have the known
   outline lengths. (`section.cut`, `gui.sections`)
5. **No install and big meshes** (T10): the engine runs in the browser (Wasm) so the app can be a static page, and
   picking and drawing stay smooth on multi-million-triangle scans. Exit: open a 5-million-triangle scan and pick in
   under 100 ms; the browser build gives the same numbers as the native one on the corpus. (`engine.wasm`,
   `gui.web`, `gui.perf`)
6. **More shapes and clean-up** (T1, T9): cone, torus, circle fits; hole fill, bad-triangle removal, reduction.
   Exit: known-answer fits within tolerance; clean-up never moves a surviving vertex more than a stated amount.
   (`fit.shapes`, `mesh.clean`)
7. **CAD out** (T8): STEP of the measured features through SolidCraft, from the app. (`cad.step`)

## Quality bar (applies to every slice)

- Numbers equal the engine's, which equal known answers (D-003); a wrong-value run for every rule.
- A first-time user finishes the slice's exit task without help; every refusal is a plain sentence.
- No new dependency without a decision line; the app stays embeddable (D-009).
- Large inputs are bounded: limits are explicit and tested.

## Risks and assumptions

- No Wasm toolchain is installed on the development machine; slice 5 needs one (Logan installs it).
- Reading CAD files as a nominal needs SolidCore; until then the nominal is a mesh.
- Nothing is proven on real scans yet; R1's exit run is the first proof and gates how much we trust the rest.
- Matching the incumbents' breadth is a long road; the order above puts the daily job first.
