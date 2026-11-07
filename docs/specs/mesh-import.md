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
- The dispatcher currently supports STL and PLY. Its limits apply to STL bytes and triangles;
  the vertex cap is checked on the STL result. The existing read_stl API remains available.

PLY description: https://sites.cc.gatech.edu/projects/large_models/ply.html (format description only).
