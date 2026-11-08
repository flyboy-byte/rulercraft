# Changelog

User-visible changes. Newest first. Versions follow SemVer; nothing is released yet (docs/distribution.md).

## Unreleased

- MCP: `thickness` keeps one spatial index per open mesh, so paging through a large selection no longer rebuilds it
  every call.
- Engine: one shared spatial index (`spatial::Bvh`) for closest point and ray casts; wall thickness no longer tests
  every triangle per ray. Results are unchanged.
- Engine: STL/PLY/OBJ/3MF reading; plane, sphere, cylinder fits (also robust); relations; wall thickness; segmentation;
  deviation with patches; ICP and datum alignment; hole patterns, symmetry and design-value guesses; CAD recipe and
  coverage check.
- CLI (`rulercraft`): info, fit, segment, project.
- MCP server (`rulercraft-mcp`): open, info, select, fit, measure, segment, compare, reverse, datum, thickness, undo/redo.
- App prototype (`rulercraft-gui`): browser-based viewer to mark, fit, measure and find regions.
