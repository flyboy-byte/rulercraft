# Commands

Status: R1 spec (outline). Features: `cli.commands`, `history.undo`.

Every user-level action is a named command with a typed JSON input and output (`mesh.open`,
`select.box`, `fit.plane`, `measure.distance`, `project.save`, `history.undo`, ...).
State-changing commands add one undo step after success; a failed command changes nothing. Read-only
commands do not change history.

The current MCP process keeps a session history of successful `mesh.open` calls. `history.undo` and
`history.redo` move between those opened meshes; `history.info` reports whether either action is
available. Opening a mesh after undo discards the redo branch. The history stores each open request and
a fingerprint of its source bytes, then reloads the file when moving through history. If the file is
missing or has changed, the move fails and the current mesh and history position are preserved. The
first undo returns the session to having no open mesh. Inspection, selection, fits and measurements
are read-only and do not add history steps. The CLI currently runs one command per process and has no
persistent history.

Engine: `rulercraft_mesh::commands` (`Selection`, `FitRequest`, `select`, `fit`), shared by the CLI and
the MCP server. Selection filters (box, near, facing) all apply; none means the whole mesh.

CLI (2026-11-07): `rulercraft info <mesh>` prints the import summary; `rulercraft fit <plane|sphere|cylinder>
<mesh> [--units] [--box] [--near] [--facing --within] [--sigma] [--known-diameter]` prints
`{mesh, selected, fit}` as JSON. `rulercraft segment <mesh> [--kind planes|cylinders|strips] [--units]`
prints compact segmentation summaries and defaults to cylinder regions for scan meshes. Errors go to
stderr with exit code 2 and nothing on stdout.
