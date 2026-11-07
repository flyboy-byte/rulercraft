# Commands

Status: R1 spec (outline). Features: `cli.commands`, `history.undo`.

Every user-level action is a named command with a typed JSON input and output (`mesh.open`,
`select.box`, `fit.plane`, `measure.distance`, `project.save`, `history.undo`, ...). The CLI and
the MCP server are thin front ends over the same command layer. Each successful command is one undo
step; a failed command changes nothing.

CLI (today): `rulercraft info <mesh>` prints the import summary as JSON.
