# Mesh to CAD

Status: *Outline*. Features: `cad.solidcraft`, `cad.check`.

From fitted elements, Claude builds the part in SolidCraft over SolidCraft's MCP server (planes as
faces, cylinders as holes or bosses, distances as dimensions). Then the mesh's points are compared
against the rebuilt solid (SolidCore projection) and the deviation reported. SolidCore is an optional
cargo feature; the core crates never depend on it (D-002).
