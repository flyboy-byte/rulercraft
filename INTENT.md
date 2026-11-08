# Intent

Locked 2026-11-07 (D-010; who-for and app updated by D-012). This page changes only through a new numbered
decision in DECISIONS.md that says what it supersedes. What v1 means: docs/v1.md. Everything else (roadmap, features, specs) serves it.

## The promise

**Any mesh in, numbers you can trust out, an editable CAD part back.** A downloaded STL, a phone scan or a
professional scan goes in. What comes out is dimensions, features and deviations, each with its method and
convention stated and checked against known answers, and a recipe that rebuilds the part as editable CAD.

## What we are building

One open engine with several faces, all giving the same numbers:

- a Rust library (and later a browser build),
- a command line for scripts and batch checks,
- an MCP server, so Claude and other agents can drive it,
- a native desktop app with a 3D view (egui and wgpu, SolidCraft's toolkit), for people who never touch an agent.

The mesh half of a mesh-to-CAD loop. The CAD half is SolidCraft and its kernel SolidCore; the engine stays
joinable with them as a library, a tool server or an embedded panel.

## Who it is for

First, the SolidCraft community: people who design in SolidCraft and print or scan parts; they judge when v1 is
finished. Then makers and engineers checking prints and scans against a design; people reverse engineering a part; agents
that need measurements they can rely on; other programs that want an engine they are allowed to embed.

## Why people choose it

1. **A permissive licence** (MIT OR Apache-2.0): it can be built into other software, closed or open,
   without a commercial negotiation.
2. **Evidence, not claims**: every number traces to a known-answer test, and results say what they assume.
3. **Scriptable and automatable**: the same checks run from the app, a script, CI or an agent.
4. **The loop closes**: measure, build the CAD, check the CAD against the mesh, in one workflow.
5. **Help with design intent**: patterns, symmetry and round-number guesses, shown with their evidence.
6. **Local and free of strings**: no account, no upload, no telemetry, no network needed.

## Principles

1. Accuracy before features. A wrong number is worse than a missing one.
2. Honest numbers: state the convention, the sample, the tolerance; refuse bad input plainly.
3. Local first. Nothing leaves the machine unless the user sends it.
4. The engine is the product; every front end is thin and adds no maths of its own.
5. Independent and permissive: new code, MIT OR Apache-2.0, no copyleft, no code or wording from other
   applications, and no other application named in this repository.
6. AI is optional: never required, never the only way in.
7. Joinable with SolidCraft, without waiting on anyone.
8. Light process, heavy evidence: small PRs, a wrong-value run for every rule, plain-language reports.

## Not our job

A CAD kernel; cloud services or accounts; sculpting or mesh art; flatbed-photo measuring (for now);
photogrammetry (research only, R6).

## How we will know

The exit conditions in ROADMAP.md, above all R1 (real published dimensions match), R2 (a part rebuilt in
SolidCraft within 0.05 mm of its scan) and R7 (a person who uses the established scan-inspection apps does
a real job here and finds nothing they need missing). Competitive scope is in docs/specs/product-scope.md.
