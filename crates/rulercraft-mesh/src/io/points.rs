// SPDX-License-Identifier: MIT OR Apache-2.0
//! Text point clouds (XYZ, PTS, ASC): one point per line, read as a mesh with no triangles.
use super::{ImportLimits, ReadError, check_bytes, vertex};
use crate::{Mesh, MeshBuilder, Unit};

/// Reads a text point cloud. Each line holds x, y and z first, separated by spaces, tabs, commas or semicolons;
/// further numbers on a line (intensity, colour, normals) are ignored. Blank lines and lines starting with `#`, `//`
/// or `;` are comments. The first line that is not a comment may be a header instead of a point: a single whole
/// number (a PTS point count) or a line with at least three fields whose first three are all nonnumeric (column
/// names). Any other line
/// that is not a point is refused with its line number.
pub fn read_text_points(bytes: &[u8], unit: Unit, limits: ImportLimits) -> Result<Mesh, ReadError> {
    check_bytes(bytes, limits)?;
    let text = std::str::from_utf8(bytes).map_err(|e| {
        ReadError(format!(
            "not text: invalid UTF-8 at byte {}",
            e.valid_up_to()
        ))
    })?;
    let mut builder = MeshBuilder::new(unit);
    let (mut points, mut first) = (0usize, true);
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("//")
            || line.starts_with(';')
        {
            continue;
        }
        let mut fields = line
            .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .filter(|f| !f.is_empty());
        let fields = [fields.next(), fields.next(), fields.next()];
        let at_start = std::mem::replace(&mut first, false);
        if at_start && matches!(fields, [Some(count), None, None] if count.parse::<u64>().is_ok()) {
            continue; // a PTS point count
        }
        let xyz = fields.map(|field| field.and_then(|f| f.parse::<f32>().ok()));
        let p = match xyz {
            [Some(x), Some(y), Some(z)] => [x, y, z],
            [None, None, None] if at_start && fields[2].is_some() => continue, // column names
            _ => {
                return Err(ReadError(format!(
                    "line {}: a point needs x, y and z numbers first",
                    n + 1
                )));
            }
        };
        points += 1;
        if points > limits.max_vertices {
            return Err(ReadError("source vertex limit exceeded".into()));
        }
        vertex(&mut builder, p, unit).ok_or_else(|| {
            ReadError(format!(
                "line {}: position or converted position is not finite",
                n + 1
            ))
        })?;
    }
    let mesh = builder.finish();
    if mesh.positions.is_empty() {
        return Err(ReadError("the file contains no points".into()));
    }
    Ok(mesh)
}
