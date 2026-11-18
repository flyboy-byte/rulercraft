# RulerCraft in SolidCraft: install and use

RulerCraft opens a 3D scan in SolidCraft as an **editable part**: a plate with its holes, every size a named
parameter you can change, and a report of anything it could not rebuild. It runs inside SolidCraft, on your
machine, with no network access.

## Install

1. Download `rulercraft-solidcraft-plugin.zip` from
   [RulerCraft's releases](https://github.com/flyboy-byte/rulercraft/releases) (the newest `plugin-v…`).
2. In SolidCraft (desktop), open **Preferences › Plugins** and choose **Install plugin…**, then pick the `.zip`.
3. The confirmation screen shows what RulerCraft asks for: no folders, no other commands, no network, and
   **1024 MiB of memory**, more than SolidCraft lets plugins use by default (128 MiB), because scans are large.
   Allow it and choose **Install**. Nothing runs until you use it.

From a terminal, the same with SolidCraft's command line (`--keep-plugin-approvals` makes the approval last):

```sh
solidcraft-cli exec plugin.install '{"path":"rulercraft-solidcraft-plugin.zip","dry_run":true}'
solidcraft-cli --keep-plugin-approvals exec plugin.install \
  '{"path":"rulercraft-solidcraft-plugin.zip","approve":{"memory_mib":1024}}'
```

## Use

- **Plugins menu › RulerCraft › scan-to-part**: choose the scan's units (millimetres unless your scanner says
  otherwise), then the file: `.stl`, `.ply`, `.obj`, `.3mf`, `.gltf` or `.glb`.
- **File › Open** keeps opening scans with SolidCraft's own readers, as a plain mesh or point cloud; use the
  Plugins menu entry to rebuild a scan as a part.
- The part opens as a new, unsaved design. Change `length`, `width`, `thickness` or a `hole…_diameter` in
  **Parameters** and the part rebuilds. Save it as any design.
- The **Import report** lists what was not rebuilt (bosses, slanted faces, holes not square to the base, an outline
  that is not a rectangle) and, under *From the importer*, the measured sizes, each hole's fit quality, and
  SolidCraft's own check: the rebuilt part's volume next to the scan's.
- Agents connected to SolidCraft's MCP server see the importer as a tool too.

From a terminal: `solidcraft-cli exec file.import '{"path":"part.stl","format":"rulercraft.scan-to-part"}'`.

## What it rebuilds today

Plate-like parts: a flat base with a rectangular outline, a parallel opposite face, and round holes square to the
base (through or blind). Anything else is listed in the report rather than guessed. Rebuilding more shapes is the
current work (docs/v1.md, phase S2). Without SolidCraft, `rulercraft solidcraft <scan> --out part.json` writes the
same design from RulerCraft's own command line.

## Remove

**Preferences › Plugins**, RulerCraft's card, **Remove**. Designs you saved keep their parts; they do not need the
plugin to open.

## When something is wrong

The report names the cause: "does not support … no face is parallel to the base" means the scan has no second
flat face to give the part a thickness. A scan that is not closed still imports, but its volume comparison means
nothing. Please open an issue at https://github.com/flyboy-byte/rulercraft/issues with the scan (if you can share
it) and the report (**Copy report**).
