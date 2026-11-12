# Changelog

User-visible changes. Newest first. Versions follow SemVer; nothing is released yet (docs/distribution.md).

## Unreleased

- Fits: 1-, 2- and 3-sigma clipping (the CLI default is 3) now uses a cut-off fixed from the all-points fit, so it
  settles instead of trimming to the round limit. On scans, clipped fits keep more points and report a larger, more
  honest sigma and form error than before (#101).
- Deviation: zero-area reference triangles no longer give a distance an unsupported sign; a reference with no
  triangle of nonzero area is refused (#102).
- Engine: tree build, deviation, alignment, symmetry and wall thickness use every core; results are identical for any
  thread count (`RULERCRAFT_THREADS` or `par::with_threads` to set it).
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
