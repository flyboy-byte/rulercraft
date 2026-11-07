# RulerCraft: context

Last updated 2026-11-06.

## North star

Hand Claude any mesh (a downloaded STL, a phone scan, a professional scan) and get back
trustworthy measurements and an editable CAD model.

## What this is

- A Rust mesh engine: read meshes, find and fit their features (planes, holes, cylinders,
  spheres), measure them, compare a mesh against a reference.
- A CLI (`rulercraft`) and an MCP server, so Claude and other agents can drive it.
- The mesh half of a mesh-to-CAD workflow. The CAD half is [SolidCraft](https://github.com/tangency-labs)
  and its kernel SolidCore (Apache-2.0): RulerCraft finds the features, SolidCraft/SolidCore build
  the solid, export STEP and project points onto the rebuilt surfaces.
- Licensed MIT OR Apache-2.0, so it can live next to SolidCraft/SolidCore.

## What this is not

- Not derived from any other mesh or scanning application, and it does not name them
  (AGENTS.md "Independence", DECISIONS.md D-001).
- Not a CAD kernel. Solids, NURBS, booleans and STEP belong to SolidCore.
- Not a photogrammetry engine yet. Photos-to-mesh is a later research direction (ROADMAP R6).
- Not a viewer or GUI yet. CLI and MCP come first.

## Who drives it

Logan (owner, decides direction, merges) with Claude and Codex doing most of the work. Logan
cannot check geometry code by eye, so every feature must be verifiable by something he can run
and read: math checks with known answers, parts with known design dimensions (ones he designs and
prints, measured with calipers), local reference numbers, and a plain-language report on every PR
(PROCESS.md).

## Direction of travel

R1 clean meshes and measurement (downloaded STLs and our own designs with known dimensions),
R2 mesh to editable CAD through SolidCraft, R3 noisy scans and deviation, R4 reverse-engineering
workflows, R5 a reusable engine others can adopt, R6 photos to mesh. See ROADMAP.md.

## Where things live

| What | Where |
| --- | --- |
| Rules for every model and contributor | AGENTS.md (outranks everything else) |
| Why decisions were made | DECISIONS.md |
| How a feature gets done, verification budget | PROCESS.md |
| Where code goes, patterns, dependencies | docs/architecture.md |
| Milestones and what is left | ROADMAP.md |
| Build order, lanes, PR contents and sizes | docs/implementation-plan.md |
| Current state | STATUS.md (generated), HANDOFF.md (last session's note) |
| What each feature must do | docs/specs/ |
| Committed test meshes and their known answers | docs/test-corpus.md, fixtures/ |
| Machine-specific notes, downloaded models, local reference numbers | `local/` and `fixtures/private/` (gitignored, never committed) |
