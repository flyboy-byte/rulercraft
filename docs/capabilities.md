# What RulerCraft supports

| Workflow | SolidCraft plugin | RulerCraft CLI | RulerCraft MCP |
| --- | --- | --- | --- |
| Rebuild a plate-like mesh | Opens an editable part and import report. SolidCraft evaluates the rebuilt volume. | The `solidcraft` command uses the same rebuild engine and writes editable design JSON. | Use the importer through SolidCraft MCP. RulerCraft MCP's `reverse` returns a recipe from a triangle mesh. |
| Read input units | Auto reads supported OBJ header units. Choose an override for OBJ, STL or PLY. 3MF and glTF carry their own conventions. | Pass `--units` where supported. 3MF and glTF carry their own conventions. | Pass `unit` on open. 3MF and glTF carry their own conventions. |
| Fit and measure a cloud | Native inspection is planned. | Fit and measure XYZ, PTS, ASC and PLY vertices. | Open, pick, calibrate and fit or measure. |
| Draft a plate with a hand-measured hole | Native assisted workflow is planned. | Individual fits and measurements are available. | `assist.plate` returns the editable design and dimension evidence. |
| Keep work for another session | Save the rebuilt design in SolidCraft. | CLI projects save supported fit recipes. | Open history is session-local. Save an assisted design and its evidence separately. |
| Rebuild an organic or branching mesh | Planned. The current importer rejects grossly mismatched plate candidates. | Same plate rebuild scope as the plugin. | No automatic organic reconstruction. |

SolidCraft 0.46 also opens E57, measures scan points and places datums through its native tools.
The immediate RulerCraft priority is using those inputs and hand dimensions to produce editable
features. See the [workflow comparison and next milestone](assisted-rebuild-next.md).

The plate path recognizes straight or circular outlines, holes, bosses, pockets, steps, windows,
and some chamfers and fillets. The importer reports omitted or simplified features. A matching
volume alone cannot establish that all features are present or correctly placed.

SolidCraft has its own **Convert Mesh** operation for a closed, consistently oriented mesh. It can
produce a faceted solid. Reconstructing a useful feature history from an open mesh such as a tree
requires a separate model of its branches and clusters. See `cad.facets`, `cad.branches`,
`sc.assisted`, and `verify.phone` in the [roadmap](../ROADMAP.md).
