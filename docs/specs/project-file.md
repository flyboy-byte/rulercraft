# Project file

Feature: `project.file`. Version 1 is a single JSON document with `version`, `source` and `elements`.
The source holds `name` (extension selects the reader), exact mesh `bytes` (array of byte values), and
`unit` (`mm`, `cm`, `m`, `in`). Embedding bytes is the simplest self-contained option: no external
mesh path/hash, zip, base64 or new hashing dependency. It preserves the original source units.

Every element has a unique nonempty `id` (at most 128 UTF-8 bytes), a typed `recipe` (kind, combined
selection filters, clipping and optional known diameter) and `last_result` (selected count and the
complete fitted result). Recipes use mm and radians internally, including facing angles. No CAD,
relations, history or segmentation is stored in version 1. These can get later format versions.

The engine receives bytes, never reads or writes files. Creating a project imports the mesh through
read_mesh and builds its graph. Adding an element fits through commands::fit; failures leave the
project unchanged. Encoding preserves the source and all recipes/results. Decoding reimports the
embedded source and refits every recipe. Cached results are never trusted as current measurements:
`changed` lists ids whose refitted typed result differs from the saved result. Comparison is exact
(including quality and count fields), not a dimensional tolerance gate; even tiny solver changes are
reported for review. Saving the reopened project records its newly computed results.

Unknown versions are refused with the version found. Invalid JSON, missing fields, invalid source
units/mesh, duplicate ids, failed recipes and limits produce actionable errors. Default project limits:
64 MiB JSON, 8 MiB source bytes, 1 million source vertices, 2 million candidate triangles, 128 elements.
Limits are explicit engine inputs. Empty projects are allowed; ids and limits are checked on add and
reopen. Unknown object fields are ignored within a known version. JSON cannot carry nonfinite numbers.

CLI:
- `rulercraft project create <project.json> <mesh> [--units mm|cm|m|in]` creates an empty project;
  refuses to overwrite an existing destination.
- `rulercraft project fit <project.json> <id> <plane|sphere|cylinder> [fit options]` reopens, fits a
  new named element and saves. Fit options are the same selections/sigma/diameter flags as standalone
  fit, except units come from the project and cannot be overridden.
- `rulercraft project info <project.json>` reopens/refits without writing, printing JSON with
  element ids, recipes/results and changed ids. Create/fit print the same summary.

Writes use a sibling temporary file then rename; a failed fit or refused input does not modify the
saved project. CLI reads enforce byte caps before parsing. This PR adds project CLI entry points;
MCP project tools and undo are separate work.
