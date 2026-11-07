# MCP server

Status: *Outline*. Feature: `mcp.server`.

A stdio MCP server (`rulercraft mcp`) exposing the command layer as tools, one tool per command, with
the command's JSON schema. Results are compact JSON; large lists are capped and say so. A short
built-in guide tells the agent the usual flow: open, inspect, segment or select, fit, measure, save.

## Stdio tools (2026-11-07)

- Run `rulercraft-mcp` (or `cargo run -p rulercraft-mcp`). It serves MCP JSON-RPC over stdin/stdout;
  stdout contains protocol messages only. The initialize instructions and `guide` describe the flow.
- One mesh is held per process. `mesh.open(path, unit)` reads local STL/PLY/OBJ/3MF (case-insensitive),
  mm by default (3MF model units win), and returns the same summary as `mesh.info`. A failed open preserves the prior mesh.
  Caller limits may lower the defaults: 2 GiB bytes, 50 million source vertices/triangles. The file
  read stops at byte limit + 1. Import/graph work and non-finite converted positions are checked
  before committing state. `mesh.info` does not reread the file.
- `select` returns sorted vertex ids for combined box, near-point and facing filters using commands.
  Parameters: selection (in_box: [min,max], near: {center,radius}, facing: {direction,within_deg}).
  Default facing angle is 30 degrees. No filters selects all vertices. Returned ids default to 256,
  maximum 1024; offset pages the list. Results give total, offset, returned and truncated.
- `fit` accepts kind plane/sphere/cylinder, selection as above, sigma 0/1/2/3 (default 3), and optional
  known_diameter as in commands. Returns selected count and compact fitted data in mm/degrees.
  At most 1 million vertices may be offered to a fit; limits are checked before the fitter runs.
- `measure` accepts first and second fit requests, fits both on the current mesh, then calls the
  engine relation function. Returns both fitted summaries, distance/components/angle_deg and the
  engine's convention. No fit or selection is persisted; only mesh.open changes process state.
- Tool execution failures are MCP tool errors with an actionable text message; invalid schemas or
  unknown tools use SDK protocol errors. Errors do not terminate the server. Saving, arbitrary explicit
  vertex lists and network transports are outside this feature.

## compare (2026-11-07)

`compare(reference, unit?, tolerance?, align?)` measures the open mesh against another mesh file without
changing the open mesh. It returns the deviation statistics (positive means outside the reference, mm),
the largest out-of-tolerance patches (at most 10, each with its vertex count and first 20 vertex ids), and,
when `align` is true, the ICP transform and residuals (the scan is moved onto the reference first; it
must already be roughly in place). Tolerance defaults to 0.1 mm and must be 0 or more. A missing or
unreadable reference is a tool error; the open mesh is untouched.
