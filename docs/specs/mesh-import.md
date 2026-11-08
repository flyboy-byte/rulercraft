# Mesh import

Status: R1 spec. Features: `io.stl`, `io.ply`, `io.obj`, `io.3mf`.

## Behaviour

- Input is bytes plus a file name (the extension picks the reader; `.stl`, `.ply`, `.obj`, `.3mf`,
  case-insensitive) and the file's unit (`mm` default, also `cm`, `m`, `in`). Everything is converted
  to millimetres on read. 3MF carries its own unit in the file and that wins.
- Output is a triangle mesh: positions (f32 triples, mm) and triangles (three vertex indices each),
  plus a short summary: vertex count, triangle count, bounding box, and which reader was used.
- Vertices that are exactly equal (bitwise after unit conversion, with -0 treated as 0) are merged
  into one vertex. Real exports repeat every shared corner once per triangle, so a 12-triangle cube
  arrives as 36 corners and must come out as 8 vertices.
- Triangles that reference the same vertex twice (degenerate after merging) are dropped and counted
  in the summary.
- Polygons with more than three corners (OBJ, PLY) are split into a triangle fan.

## STL

- ASCII if the file starts with `solid` **and** parses as ASCII; otherwise binary. Some binary files
  start with the word `solid` in their 80-byte header; they must still read as binary when the
  triangle count in bytes 80-83 matches the file length (84 + 50 per triangle).
- The facet normal in the file is ignored; normals are recomputed from the geometry.
- A binary file whose length disagrees with its triangle count is refused, with a message that says
  how many triangles the header claims and how many bytes are present.

## Refusals (all return an error, never panic)

- Empty file, or no triangles.
- Unknown extension, with the list of supported ones.
- A number that is not finite (NaN, infinity) anywhere in the positions.
- Over the limits: default 2 GiB of input, 50 million triangles. Limits are parameters.
- Truncated or malformed data, with the byte offset or line number where reading stopped.

## Checks

- Math: a unit cube in ASCII and binary gives 8 vertices, 12 triangles, box (0,0,0)-(1,1,1).
- Known design: our designed test block reads with the box its design states.

## PLY (2026-11-07)

- Read version 1.0 ASCII, binary little-endian and binary big-endian. Header lines end in LF
  or CRLF. Comments and obj_info are ignored. Scalar types: signed/unsigned 8/16/32-bit integers
  and 32/64-bit floats, using standard names or width aliases.
- Exactly one vertex element precedes exactly one face element. Vertex x/y/z are scalar
  properties in any order; face vertex_indices (or vertex_index) is an integer list.
  Other declared properties and elements are consumed and ignored. Duplicate elements or
  properties, missing coordinates, unsupported types and malformed declarations are refused.
- Face indices are zero-based. Faces need at least three indices. Fan triangulation preserves
  their order; it does not repair concave or self-intersecting polygons.
- ASCII body values are whitespace separated, with line numbers in refusals. Binary refusals
  give byte offsets. Undeclared trailing data is refused (ASCII trailing whitespace is allowed).
- ImportLimits bounds bytes, source vertices (default 50 million), and candidate triangles
  (default 50 million, including dropped degenerates). List lengths are checked against remaining
  input before allocation. Non-finite positions and unit conversion overflow are refused.
- STL triangle limits count candidate triangles, including degenerate fan triangles, and stop ASCII
  decoding as soon as the cap is exceeded.
- The dispatcher supports STL, PLY and OBJ. Its limits apply to STL bytes and triangles;
  the vertex cap stops STL decoding when the merged vertex count exceeds the limit. The existing
  read_stl API remains available.

PLY description: https://sites.cc.gatech.edu/projects/large_models/ply.html (format description only).

## OBJ (2026-11-07)

- Geometry-only text reader: `v x y z` (optional weight must be 1), `f` polygons, comments
  beginning with #, LF/CRLF, and backslash line continuations. Vertex colors are not supported.
- Face corners accept v, v/vt, v//vn or v/vt/vn. Position indices are positive one-based or
  negative relative to vertices already declared; zero, forward and out-of-range references refuse.
  Texture/normal indices must be nonzero integers but their targets are ignored in this geometry pass.
- `vt`, `vn`, `vp`, `o`, `g`, `s`, `mtllib`, `usemtl`, `l`, `p` are ignored. Other statements
  refuse with a line number; curves and surfaces are not polygon geometry.
- Faces need at least three corners; ordered fans do not repair concave polygons. All source
  positions, including unused ones, are validated and retained through MeshBuilder. Limits count
  source vertices and candidate triangles, including degenerates. Invalid text and numeric data,
  unsupported weights and unit overflow refuse with a line number (byte offset for invalid UTF-8).

## 3MF (2026-11-07)

- Geometry-only core model import from stored/deflated ZIP. `_rels/.rels` selects the primary model
  by its standard start-part relationship. No archive extraction or external targets. Required
  extensions refuse; optional appearance, materials, metadata and nongeometry parts are ignored.
- UTF-8 XML, default or prefixed namespaces, quoted attributes, predefined/numeric entities,
  comments and processing instructions. DTD/custom entities refuse. Errors identify ZIP context or
  model XML byte offset. This is a geometry importer, not a complete package/schema validator.
- Model units win over the supplied unit: micron, millimeter (default), centimeter, inch, foot, meter.
  Resource objects contain a mesh or components; build items and nested same-model components are
  flattened with their affine 12-number transforms. Negative determinant reverses triangle winding.
  Unsupported external model references refuse, including optional path attributes on instances.
- All source positions/indices and transforms are validated, including unused objects. Unique positive
  resource ids, existing object references and acyclic components are required. Only build items are
  emitted; `printable="false"` items are skipped. MeshBuilder merges vertices and drops degenerates.
- ImportLimits bounds compressed bytes, combined uncompressed relationship/model bytes, total declared
  source geometry and expanded geometry (vertices/triangles counted before merging/dropping).
  Additional fixed caps: 10,000 ZIP entries, 100,000 resources/instances, XML/component depth 64.
  A single relationship selects the model; absent/ambiguous relationships, no build triangles,
  bad indices, nonfinite coordinates/transforms and f32 overflow after transforms/units refuse.
- Simplest initial scope: one model part; no manufacturing/material semantics, mesh repair, URI percent
  escapes or cross-part components. The existing dispatcher and `info`/`fit` accept `.3mf`.

Format description: https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md

## CLI file loading (2026-11-07)

CLI info/fit check a regular file's declared byte size before allocating its contents, then read at
most the default byte cap plus one (also catching files that grow after the metadata check). An
oversized file reports its path and byte limit, not a late allocation error. The same bounded-read
helper is used by project commands. Existing engine byte limits still validate the supplied bytes.
