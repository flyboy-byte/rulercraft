# Standalone GUI

Status: *First slice built (2026-11-07)*. Feature: `gui.viewer`. Decision: D-009.

`rulercraft-gui [mesh] [--units U] [--port N] [--no-open]` starts a small server on 127.0.0.1 only and
opens the page in the default browser. Nothing is uploaded anywhere; it works with no Claude and no
internet. It is the same engine as the CLI and MCP server, with a face.

## What you can do
1. **Open** a mesh (button, drag-and-drop, or the command line). Formats and limits as in mesh-import.md.
2. **Look**: a shaded 3D view (drag to turn, right-drag to move, wheel to zoom, F or "Reset view" to refit).
3. **Mark**: click to paint a patch (a ball of the chosen radius around the clicked point); Shift/Alt/Ctrl-click
   removes; "Mark all" and "Clear". The marked vertices show as orange points.
4. **Fit** the marked vertices to a plane, sphere or cylinder, with the stray-point setting (3σ default,
   2σ, 1σ, keep all) and an optional designed diameter. The numbers appear in plain words (diameter,
   length, axis, flatness, σ) and the fitted shape is drawn over the part.
5. **Measure** between two ticked fits: distance and/or angle, with the convention in words.
6. **Find regions** automatically (flat faces or round surfaces), shown in colours; with "click marks the
   whole region", one click marks every vertex of a region, ready to fit.

## How it is built
- `crates/rulercraft-gui`: a library (`serve`, `State`) plus the `rulercraft-gui` binary, standard library
  only (a small HTTP/1.1 server, one thread per connection). The page is `assets/index.html`,
  `assets/app.js` (hand-written WebGL2) and `assets/geom.js` (picking, brush and camera maths, also run
  under Node). No framework, no JavaScript library, no CDN, no new Rust dependency.
- The page loads the mesh as raw buffers (`/api/positions` float32, `/api/indices` uint32) and asks the
  server for numbers: `/api/info`, `POST /api/open?name=&unit=` (raw file bytes), `POST /api/fit`
  (`kind`, `vertices`, `sigma`, `known_diameter`), `POST /api/measure` (two fit requests) and
  `POST /api/segment` (`kind`: planes or cylinders; returns a region id per triangle and region summaries).
  Fit and measure results use the same engine functions and conventions as the MCP server.
- Errors are plain sentences with HTTP 400; a failed open keeps the previous mesh; the server never reads a
  path the page supplies, only bytes.

## Safety
Loopback only. The Host header must be `127.0.0.1:<port>` or `localhost:<port>` (stops DNS rebinding) and an
Origin header, when present, must be the page's own origin (stops another website's page from sending
commands). Bodies are capped (512 MiB), fits at 1 million vertices, ids are checked (a huge number is refused,
not wrapped). Responses are not cached and carry `nosniff`.

## Joining SolidCraft (D-009 addendum)
The GUI is a thin client over the engine. A host program can use the engine as a library (or later as Wasm), the
MCP server, or `cad::recipe`. The page is self-contained and embeddable (one origin, no frame-blocking headers,
no assumption of being the top-level window). Nothing is built against an assumed SolidCraft plugin interface.

## Checked
- Server over a real socket (7 tests): page and buffers, fit/measure/segment numbers on a known cube, plain
  errors for bad input, the Host and Origin refusals, opening from the browser (path stripped, bad file keeps the
  old mesh, units applied), the stray-point setting. Wrong-value runs: Host check off, Origin check off, units
  ignored on open, id cast instead of checked, stray-point setting ignored: each failed a test.
- Viewer geometry under Node (`tests/geom.test.js`, run by `cargo test` when `node` is installed): ray vs
  triangle, camera ray through the screen centre and right side, matrix inverse, brush, circle and basis.
- By hand, in headless Chrome driven over its debugging port with real mouse events, on a 40 x 30 x 6 mm plate with
  a 16 mm boss: regions found; a click marked the plate top (4 vertices) and the plane fit gave 40.000 x 30.000
  mm with normal (0, 0, 1); a click on the round region marked 96 vertices and the cylinder fit gave diameter
  16.000, length 14.000, axis (0, 0, 1); measuring them gave 90.000°; a brush click marked 41 vertices,
  Shift-click removed them, and orbiting marked nothing. This hand run is not an automated test.

## Not done
Exporting the CAD recipe and the deviation/ICP views from the page; wall thickness view; undo; saving and
loading projects from the page; point-to-point rulers; a native window (would need a graphics dependency, ask
first); performance on meshes beyond a few million triangles (picking is a CPU loop and the draw buffers are
expanded per triangle); trying it inside a SolidCraft panel.
