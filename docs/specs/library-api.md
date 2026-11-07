# Library API

Status: *Partly done (2026-11-07)*. Feature: `engine.library`.

The engine is the `rulercraft-mesh` crate. This page says what a program can rely on.

## What exists
- Crate documentation (`cargo doc -p rulercraft-mesh`) opens with a worked example that is run as a test:
  read a mesh from bytes, build its graph, fit a plane. If the example breaks, the build fails.
- `rulercraft_mesh::prelude` re-exports the names most programs need.
- Every public item is documented (`missing_docs` is a warning and CI treats warnings as errors).
- No `unsafe` code; no panics on bad input (errors are returned); results are deterministic, including the
  random-sampling fits (seeded).

## Stability promises (before 1.0)
- Stable in intent, changeable with a version bump: reading (`read_mesh`, `ImportLimits`, `Mesh`), the
  fit functions and their result structs, `commands`, `measure::relate`, `deviation`, `align`.
- Expect change: `segment` options, `reverse`, `cad` (their results are guesses and the shapes may grow
  fields), and `project` until the file format is frozen.
- Lengths are millimetres and angles are degrees in every result, except `Selection::facing`, which takes
  radians (a known wart; the MCP and CLI layers convert).

## Not done
- Publishing to crates.io (needs Logan's go-ahead and a name check).
- A WebAssembly build: `rulercraft-mesh` has no system dependencies, but the build has not been tried and
  no wasm target is installed on the development machine.
- Frozen module layout and a changelog.
