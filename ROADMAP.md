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

## R7 v1 in SolidCraft: the plugin, at parity and beyond

**Exit:** a SolidCraft user (Logan standing in until others try it) does all four jobs of docs/v1.md on their own
parts inside SolidCraft with the RulerCraft plugin, without help.

SolidCraft is the user interface (D-015); RulerCraft ships as its plugin (D-014). In order: the importer (done), a
release people can install (built, waiting on a tag), real parts (with R1, still owed), more shapes rebuilt (outline,
bosses, steps and details: done on designed parts), a deviation check in the import report (done), inspecting scans
inside SolidCraft (blocked on SolidCraft's mesh-size limit), and Claude driving it through SolidCraft's MCP (done,
docs/agents-and-solidcraft.md).
Features `sc.*`; phases in docs/v1.md.

## Not planned (say so before starting)

A CAD kernel of our own, cloud services or accounts, and a 2D flatbed-photo measuring workflow.

## Measurement-assisted phone capture to editable CAD

Added 2026-11-21. A phone export and a few physical measurements are enough to attempt a first draft.
The first acceptance part is a rectangular plate with one circular hole. The caller can identify the
reference points and visible features. RulerCraft must ask for missing dimensions or decisions when the
scan cannot support them.

1. **R6 / scan.scale and scan.picks.** Prove that file origin does not silently change a hand reference.
   Return actual selected coordinates so the caller can inspect anchors and supports.
2. **R4 / cad.assisted.** Bind the selected supports and hand readings to an editable plate and hole.
   Hold out width and hole center and verify an edit in SolidCraft. Include a missing-bore case.
3. **R7 / sc.assisted and verify.phone.** Carry dimension sources into the SolidCraft workflow and run
   the same acceptance case on an untouched phone export with held-out caliper readings.

See the [acceptance matrix](docs/specs/assisted-rebuild.md) and [controlled trials](docs/assisted-phone-rebuild.md).
