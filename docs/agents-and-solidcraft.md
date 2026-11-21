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

## Rebuild a plate from a point cloud

Use RulerCraft's MCP server for point selection, calibration and the assisted draft. SolidCraft
opens the resulting design for editing.

1. Open the cloud with `mesh.open`. For PLY vertices, pass `as_points: true`. Use `select` with
   `include_positions: true` to inspect the coordinates and IDs of candidate reference points.
2. Reopen with `calibration` using the two reference IDs and the measured `distance_mm`. A second
   physical span can go in `calibration.check`. Recheck selections in calibrated millimetres.
3. Call `assist.plate` with three top-corner IDs in origin, length, width order, the plate thickness
   and hole diameter, a center, and a depth choice. For example, given picked IDs 0, 1 and 2
   and a measured through-hole:

   ```json
   {
     "corners": [0, 1, 2],
     "thickness_mm": 8,
     "diameter_mm": 8,
     "center_mm": [15, 12],
     "depth": {"through": true}
   }
   ```

4. Save the returned `design` object as a SolidCraft project and open it in SolidCraft. Retain the
   `evidence`, `frame`, and `calibration` separately to trace the inputs. Check the resulting dimensions
   before editing or using the part.

Provide `width_faces` to refine the width from opposing side patches. A selected `bore_selection` and
`max_diameter_difference_mm` can replace the explicit center. When bore evidence is weak, provide a
measured center or pick a better wall region. For a blind hole, provide `depth.blind_depth_mm`.
The [assisted rebuild spec](specs/assisted-rebuild.md) gives the complete contract and trial results.
