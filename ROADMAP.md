# Roadmap

Last updated 2026-11-07. Each milestone ends in something Logan can run and check, not just "tests
pass". Features and their state live in `plan/features/`; the generated view is ROADMAP-PROGRESS.md.
Completed items are struck through with a date, never deleted.

## R1 Clean meshes: open, fit, measure, over CLI and MCP

**Exit:** Logan downloads an STL from a model-sharing site, asks Claude over MCP for its hole diameters and face
distances, and they match the design dimensions.

Readers (STL, PLY, OBJ, 3MF), the mesh model, selection, plane/sphere/cylinder fits, automatic
segmentation into surfaces, distances and angles, a project file, undo, the CLI and the MCP server.
In parallel: the known-design corpus (our own designed test parts, committed; downloaded models with
published dimensions, kept local; a test block Logan prints and measures) and local reference numbers.

As of 2026-11-07: ~~readers, fits, relations, MCP, engine segmentation, corpus, project JSON and local
reference helper~~ are merged. Undo/redo and segmentation/project MCP tools remain. Checkpoint #32
compares all 41 corpus dimensions/relations through MCP and repeats the private-scan checks; this
is not the published-dimension or printed/caliper acceptance, which remains open.

## R2 Mesh to editable CAD through SolidCraft

**Exit:** a simple prismatic downloaded part is rebuilt as a SolidCraft model whose deviation from the
STL is under 0.05 mm.

Claude turns fitted features into SolidCraft operations over SolidCraft's MCP server; then the mesh is
checked against the rebuilt solid (SolidCore projection, optional feature).

## R3 Noisy scans

**Exit:** a real scan (a public sample scan or a phone scan) gives stable fits and a deviation report against
its CAD.

Robust fitting, mesh-to-mesh deviation, alignment (datums, ICP), wall thickness.

## R4 Reverse-engineering workflows with AI

**Exit:** Claude proposes a feature tree (holes, patterns, symmetry) for a part and Logan accepts it in
SolidCraft.

## R5 Reusable engine

**Exit:** another project depends on `rulercraft-mesh` and builds for Wasm.

## R6 Photos to mesh (research)

**Exit:** a written go/no-go with a prototype on photos of one object. Any library used must pass the
licence rules in AGENTS.md (no copyleft).

## R7 A tool people choose: the app, at parity and beyond

**Exit:** Logan, who uses the established scan-inspection apps, does a real job in the RulerCraft app (a
dimensioned report, a deviation check against the design, an alignment) and finds nothing he needs missing.

The standalone app brought up to the everyday job (constructions, dimensions with tolerances, deviation map,
alignment, sections, reports, save and undo, big meshes, no-install browser build) while keeping what we win on:
permissive licence, evidence-backed numbers, scripting, the closed loop to CAD. Scope and order of work:
docs/specs/product-scope.md. Intent: INTENT.md (D-010).

## Not planned (say so before starting)

A CAD kernel of our own, cloud services or accounts, and a 2D flatbed-photo measuring workflow.
