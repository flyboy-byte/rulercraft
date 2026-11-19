# Changelog

User-visible changes. Newest first. Versions follow SemVer; nothing is released yet (docs/distribution.md).

## Unreleased

- Scan to SolidCraft on real scans: sizes come from the walls (a scan's faces stop short of their softened edges),
  holes and bosses keep their real depths and heights, softened edges are summarised in one report item instead of
  hundreds, and a noisy 134,000-triangle scan takes 3 seconds, not a minute. Checked on a test part meshed like a
  scan, turned and with noise: every size within 0.05 mm.

- Faster and steadier on real scans: finding round regions no longer slows to minutes on a dense scan (a 134,000
  triangle scan of the bracket went from over 8 minutes to 3 seconds), and a scanned plate's axes follow its walls
  instead of the ragged edge of its base face, which had turned the frame by 0.7° and grown every size.

- Scan to SolidCraft: rounded top edges are rebuilt as fillets with a named radius, corner patches included. A
  plate's thickness no longer reads slightly thin when its top edges are rounded.

- Scan to SolidCraft: countersinks are rebuilt as cone cuts on their holes, and bevels on the top edges of the
  outline as chamfers with named sizes. Both checked against SolidCraft-made parts (exact volumes) and follow
  `thickness` when it is edited.

- Scan to SolidCraft: through holes, holes blind from the top and bosses on the top face now follow `thickness`
  when it is edited (a boss used to stay where it was). The plugin's volume comparison moves to
  `details.volume_check`, so it no longer hides the point-by-point `details.check`. New `tools/agent_smoke.py` and
  docs/agents-and-solidcraft.md: Claude installing and driving the importer over SolidCraft's MCP.
- Scan to SolidCraft: the import report now measures the scan against the rebuilt part, point by point (mean, RMS,
  95th percentile, worst, share within tolerance), and says so when the rebuild is off the scan.

- Scan to SolidCraft: windows (non-round cutouts through the plate) are rebuilt as cut sketches; a rectangle of four
  corners is no longer mistaken for a circle. Checked against SolidCraft: 8,280 mm³ exact.
- Scan to SolidCraft: counterbored holes are checked against a SolidCraft-made part (11,377.965 mm³, exact): the wide and narrow bores come back as two coaxial cuts.

- Scan to SolidCraft: a plate with any outline (L-shapes, round, angled, with cut corners) is rebuilt as its traced
  outline, not its enclosing rectangle; a round plate is a circle. A stepped cylinder now comes back at its true volume
  (4,272.566 mm³; the old box over its circle gave 5,121 mm³).

- Scan to SolidCraft: bosses, pockets, steps and pads are rebuilt too, and the plate lines up with its outline's
  edges and takes its thickness from the largest parallel face (before, a tilted frame and a boss's top gave wrong
  sizes). A bracket designed in SolidCraft comes back with the same volume to 11 digits (#112).
- SolidCraft is RulerCraft's user interface (D-015): the plan for a separate native app is dropped. The plugin is
  released as a `.zip` on GitHub releases (tags `plugin-v…`); install guide in docs/install-solidcraft-plugin.md (#111).
- Selection: `--facing` (and MCP/app facing) also counts a vertex when a triangle touching it faces the direction,
  so a box side on a sharp-edged CAD mesh can be selected by facing. Facing selections on scans can gain edge
  vertices of the faced surface (#110).
- SolidCraft plugin: `plugin/solidcraft` builds RulerCraft as a SolidCraft importer (Plugins › RulerCraft ›
  scan-to-part): a scan opens as an editable part with an import report, checked by SolidCraft's own rebuild (#109).
- Scan to SolidCraft: `rulercraft solidcraft <mesh> --out design.json` rebuilds a scanned plate as an editable
  SolidCraft design (box, cut holes, named sizes) with a report of everything it could not rebuild (#108).
- CLI: `rulercraft measure` fits two selections and prints their distance and angle with the convention used, the
  same numbers as the MCP measure tool. Cylinder fits on the CLI now also report `extend_before`/`extend_after` (#107).
- Reading: glTF 2.0 (`.gltf` with embedded buffers) and GLB, the usual phone-scan export, with node transforms;
  positions are metres by the standard. Compressed (Draco) files and buffers kept in separate files are refused
  with a message saying what to export instead (#106).
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
