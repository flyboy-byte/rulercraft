# Driving the importer from an AI agent

SolidCraft's MCP server (`solidcraft-cli mcp`, see SolidCraft's docs/mcp.md) runs the same commands as its menus, so
an agent such as Claude can install the RulerCraft plugin, open a scan as a part, read what was lost, and edit the
part, with no window.

```sh
python3 tools/agent_smoke.py rulercraft-solidcraft-plugin.zip fixtures/known/bracket.stl solidcraft-cli
```

That script is the whole path, over MCP's JSON-RPC, and it fails on the first step that does not work:

1. `execute plugin.install {"path": ZIP, "approve": {"memory_mib": 1024}, "replace": true}`.
2. `execute file.import {"path": SCAN, "format": "rulercraft.scan-to-part"}`. The answer carries `loss_report`:
   `summary`, `items` (what was dropped or simplified, each with a `code`), and `details` (`plate`, `holes`, `bosses`,
   `pockets`, `pads`, `cutouts`, `check` for the scan's distance from the part, `volume_check` for the volume
   compared in SolidCraft).
3. `set_parameter {"name": "thickness", "value": 10}`: every size is a parameter (`length`, `width`, `thickness`,
   `hole1_diameter`, `boss1_height`, ...), and holes and bosses on the top face follow `thickness`.
4. `measure`, `export`, `screenshot` and the rest of SolidCraft's tools work on the part as on any design.

What an agent should do with the report: read `items` first; anything `dropped` is missing from the part, anything
`degraded` is simplified; `details.check.p95_mm` says how far the rebuilt part sits from the scan; then tell the
person which features need modelling by hand.

Plugin tools (commands) are listed to agents as `<plugin>.<entry>`; the importer is a file format, so it runs
through `file.import`, not a tool of its own.
