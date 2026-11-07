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
