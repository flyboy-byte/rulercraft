# Commands

Status: R1 spec (outline). Features: `cli.commands`, `history.undo`.

Every user-level action is a named command with a typed JSON input and output (`mesh.open`,
`select.box`, `fit.plane`, `measure.distance`, `project.save`, `history.undo`, ...). The CLI and
the MCP server are thin front ends over the same command layer. Each successful command is one undo
step; a failed command changes nothing.

Engine: `rulercraft_mesh::commands` (`Selection`, `FitRequest`, `select`, `fit`), shared by the CLI and
the MCP server. Selection filters (box, near, facing) all apply; none means the whole mesh.

CLI (2026-11-07): `rulercraft info <mesh>` prints the import summary; `rulercraft fit <plane|sphere>
<mesh> [--units] [--box] [--near] [--facing --within] [--sigma] [--known-diameter]` prints
`{mesh, selected, fit}` as JSON. Errors go to stderr with exit code 2 and nothing on stdout.
