# Roadmap

Last updated 2026-11-06. Each milestone ends in something Logan can run and check, not just "tests
pass". Features and their state live in `plan/features/`; the generated view is ROADMAP-PROGRESS.md.
Completed items are struck through with a date, never deleted.

## R1 Clean meshes: open, fit, measure, over CLI and MCP

**Exit:** Logan downloads an STL from a model-sharing site, asks Claude over MCP for its hole diameters and face
distances, and they match the design dimensions.

Readers (STL, PLY, OBJ, 3MF), the mesh model, selection, plane/sphere/cylinder fits, automatic
segmentation into surfaces, distances and angles, a project file, undo, the CLI and the MCP server.
In parallel: the known-design corpus (our own designed test parts, committed; downloaded models with
published dimensions, kept local; a test block Logan prints and measures) and local reference numbers.

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

## Not planned (say so before starting)

A GUI or viewer, a CAD kernel of our own, and a 2D flatbed-photo measuring workflow.
