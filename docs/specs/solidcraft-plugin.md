# SolidCraft plugin

Status: first version. Feature: `cad.solidcraft`. Decision: D-014.

RulerCraft ships to SolidCraft as an **importer plugin** built from `plugin/solidcraft`, against SolidCraft's
published plugin interface (its `docs/plugins.md`, `docs/plugin-runtime.md`, and the book's "Writing a plugin";
WIT package `craft:plugin@0.2.0`, copied into `plugin/solidcraft/wit/` with its MIT notice).

## What the user sees

In SolidCraft, **Model › Plugins group › RulerCraft** opens the importer. The group's menu lists it
as **RulerCraft…**. Its dialog asks for the scan's units (mm by default), then **Choose File…** picks
the input (`.stl`, `.ply`, `.obj`, `.3mf`, `.gltf`, `.glb`). The scan opens as a new, unsaved
design holding the part RulerCraft rebuilt (docs/specs/mesh-to-cad.md, "SolidCraft design"): a plate with its
holes cut, every size a named parameter. The Import report window lists what was not rebuilt. SolidCraft reads every
one of these formats itself (as a mesh, or PLY as a point cloud), so File › Open keeps doing that; the plugin is
chosen from its menu entry, or with `file.import` (always an importer; `format: "rulercraft.scan-to-part"` when
another importer claims the extension too).

## What it does

1. Reads the file through `source.read` in 1 MiB chunks (at most RulerCraft's import byte limit), checking
   Cancel between chunks and reporting progress.
2. Reads the mesh with RulerCraft's readers in the chosen units, and rebuilds it with `solidcraft::design`.
   A scan that cannot be rebuilt (no flat face, no thickness) fails as `unsupported`, saying why; a broken file
   fails as `invalid-input`.
3. Writes the design through `design.write`.
4. Checks it (`evaluate@0.2`): SolidCraft opens the design in a private session and `inspect.measure` gives its
   volume in the report's `details.volume_check`. The engine reports a scan volume only if each edge is shared by two
   triangles traversing it in opposite directions and the unsigned result is positive and finite. Otherwise the
   scan volume and difference percentage are null, with a note explaining the requirement. The rebuilt volume
   is still shown when SolidCraft can measure it. A failed host check is reported there and does not fail the
   import. The report's `details.check` is the
   separate point-by-point deviation of the scan from the rebuilt part (mesh-to-cad spec).

## Package

`plugin/solidcraft/package/plugin.json`: id `rulercraft`, capabilities `formats@0.2` and `evaluate@0.2`, one
importer entry `scan-to-part`, no commands, no folder grants, no network. Limits: fuel 10¹¹ (SolidCraft's default
ceiling), 1024 MiB memory (above the 128 MiB default ceiling, so the install screen asks the user to allow it),
10 minutes, 16 MiB stack.

The crate is its own Cargo workspace, the one place that depends on `wit-bindgen` (D-014). wit-bindgen's
generated ABI glue is the only `unsafe` code, confined to two modules; `unsafe` is denied everywhere else.

## Building, checking, testing

`plugin/solidcraft/build.sh` builds for `wasm32-unknown-unknown`, wraps the module with `wasm-tools component new`,
refuses a component that imports WASI or network interfaces, and zips `plugin.json` and `rulercraft.wasm` into
`plugin/solidcraft/dist/rulercraft-solidcraft-plugin.zip`. With `solidcraft-cli` available it then runs SolidCraft's
own `plugin check` and `plugin test` on the package.

`plugin test` runs `package/tests/*.json` through SolidCraft's real `file.import`, with known answers: the hole
block rebuilds as one body of 17,230.3098 mm³ (and 1,000 times that read as centimetres), the bracket as its
designed 31,282.2124 mm³, the stepped cylinder as 5,121.3791 mm³ (its rectangle stands in for the round outline),
and a lone sheet is refused as `unsupported` naming the thickness.

The workflow `.github/workflows/solidcraft-plugin.yml` builds the package and uploads it as an artifact. It is not
part of the fast gate: it runs when started by hand or on a pull request that changes `plugin/`.

## Not yet

Command entries (measure or fit a mesh body already in a design); bosses, non-rectangular outlines, slanted
faces and tilted holes (they go to the report); a signed or published package.

## Checked (2026-11-18)

With SolidCraft's CLI built from tangency-labs/solidcraft 4711df6a and the component built from this crate:
`plugin check` reports no errors (one warning: the 1024 MiB memory request, which Install asks the user to
allow); `plugin test` passes all four cases; `plugin.install` (with `approve: {memory_mib: 1024}`) then
`file.import` of the hole block gives the report above with no items, and SolidCraft's private rebuild measures
17,230.3098 mm³ against the scan's enclosed 17,231.5432 mm³ (−0.007 %, the scan's holes being polygons). The
Khronos sample `Box.glb` opens as a 1,000 mm cube, equal to the scan's volume to 1e-14 %; `file.import` always
uses an importer plugin, so with RulerCraft the only one claiming `.glb` it is picked without naming `format`.
