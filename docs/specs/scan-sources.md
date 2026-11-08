# Scan sources: anything that makes a mesh

Status: *Scoping (2026-11-07)*. Milestone R6 (research first). Serves INTENT.md: "any mesh in".

People get geometry in many ways. RulerCraft should accept all of them, say honestly how accurate each kind usually
is, and turn each into a mesh the rest of the engine can measure. This page maps the sources, the pipeline, the file
types, and the order of work. Statements about outside libraries are marked *(check)*: they must be verified (licence
and behaviour) before anything is used, and D-001 still applies (no GPL, LGPL or AGPL code; a new dependency needs a
decision line).

## Sources

| Source | What it gives | Typical scale and accuracy | Our job |
| --- | --- | --- | --- |
| Downloaded models (STL, 3MF, OBJ) | exact designed triangles | exact, in the file's units | read; units; already done |
| Desktop and handheld 3D scanners (structured light, laser) | dense meshes or point clouds | sub-millimetre for good scanners | read their exports; point clouds to mesh; clean-up |
| Phones with depth sensors (some Android phones with time-of-flight, phones with LiDAR) | depth frames, coloured meshes | roughly millimetres to a centimetre *(check per device)* | read the exports of capture apps first; a capture app of our own later |
| Phones without depth sensors (most Android phones) | photos, plus depth estimated from motion | depends on method; needs a known scale | photos to mesh (below) |
| Photos (any camera) | images | needs a scale reference (a ruler, a printed target) | photogrammetry pipeline (research) |
| CT and other volume scans | voxel volumes | sub-millimetre | volume to surface (later) |
| Point-based captures (radiance-field or point-splat exports) | coloured points with size | varies | points to surface (research) |

## The pipeline

capture → align the views → points or depth → surface → clean-up → **scale** → measure.

Each step we own or wrap gets the same proof as everything else: generated scenes with known geometry (render a known
part, reconstruct, measure, compare) and the regression corpus.

- **Scale** is the step people forget: a photo set has no size until something of known size is in it. Plan: detect a
  printed target or let the user mark a known distance; every result from a scaled source states the scale's source.
- **Uncertainty**: results from scans carry the fit residuals already; results from photos also carry the scale
  reference's uncertainty.

## File types

| Kind | Formats | State |
| --- | --- | --- |
| Meshes | STL, PLY, OBJ, 3MF | read today |
| Meshes, more | glTF/GLB (common phone-app export), OFF, AMF, VRML | planned (`io.gltf`, others on demand) |
| Point clouds | PLY points-only, XYZ/PTS/ASC text, E57 (ASTM standard for scanners), LAS/LAZ, PCD | planned (`io.pointcloud`, `io.e57`) |
| CAD as a nominal | STEP | through SolidCore later (D-002) |
| Phone formats | USDZ (Apple) | research; may need a dependency |

## Points to surface

Needed by scanners that export clouds, by depth sensors and by photogrammetry. Candidate textbook methods: Poisson
surface reconstruction, ball pivoting, and fitting our known primitives directly to points (we can measure many features
without ever making a mesh). Start with fitting directly to points (we have the fits), then a reconstruction method.

## Photos to mesh (research, R6)

Goal of the research: a written go/no-go with a prototype on photos of one object (ROADMAP R6 exit).
Questions to answer: which open, permissively licensed pipeline is good enough *(check: some well-known ones are BSD,
some MPL, some AGPL; MPL is weak copyleft and needs a decision)*; whether to run it as a separate program we call
(cleaner licence boundary) or link it; what accuracy a phone and a printed scale target reach on a known part; what a
user must do (how many photos, lighting, a turntable).

## Android and phone capture (research)

Two routes: (1) read what existing capture apps export (glTF, OBJ, PLY, USDZ), which needs no app of our own; (2) our
own capture app later, using the platform's depth APIs *(check licences and terms)*, which sends frames to the engine.
Route 1 first.

## Interfaces

Every new source is reachable from the library, CLI, MCP and app with the same options, and agents can drive the whole
pipeline over MCP.

## Order of work

1. Point clouds in (PLY points, XYZ, E57) and fitting directly to points.
2. glTF/GLB in (phone apps).
3. Points to surface (one method, proven on generated scenes).
4. Photo research go/no-go (R6).
5. Our own phone capture, only if route 1 leaves real gaps.
