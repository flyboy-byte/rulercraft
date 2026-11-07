// SPDX-License-Identifier: MIT OR Apache-2.0
//! Geometry-only OBJ text import.
use super::{ImportLimits, ReadError, check_bytes, finish, vertex};
use crate::{Mesh, MeshBuilder, Unit};

/// Reads polygon geometry with bounded source vertices and candidate triangles.
pub fn read_obj(bytes: &[u8], unit: Unit, limits: ImportLimits) -> Result<Mesh, ReadError> {
    check_bytes(bytes, limits)?;
    let text = std::str::from_utf8(bytes)
        .map_err(|e| ReadError(format!("byte offset {}: invalid UTF-8", e.valid_up_to())))?;
    let mut builder = MeshBuilder::new(unit);
    let mut ids = Vec::new();
    let mut triangles = 0usize;
    let mut logical = String::new();
    let mut start = 1;
    for (n, line) in text.lines().enumerate() {
        if logical.is_empty() {
            start = n + 1;
        }
        let content = line.split('#').next().unwrap_or("").trim_end();
        if let Some(part) = content.strip_suffix('\\') {
            logical.push_str(part);
            logical.push(' ');
            continue;
        }
        logical.push_str(content);
        let error = |s: &str| ReadError(format!("line {start}: {s}"));
        let mut words = logical.split_whitespace();
        match words.next() {
            None => {}
            Some("v") => {
                if ids.len() >= limits.max_vertices || ids.len() >= u32::MAX as usize {
                    return Err(error("source vertex limit exceeded"));
                }
                let mut p = [0.; 3];
                for v in &mut p {
                    *v = words
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .ok_or_else(|| error("vertex needs three numbers"))?;
                }
                if let Some(w) = words.next() {
                    if w.parse::<f64>() != Ok(1.) || words.next().is_some() {
                        return Err(error("only optional vertex weight 1 is supported"));
                    }
                }
                ids.push(
                    vertex(&mut builder, p, unit)
                        .ok_or_else(|| error("position or converted position is not finite"))?,
                );
            }
            Some("f") => {
                let mut face = Vec::new();
                for word in words {
                    let parts: Vec<_> = word.split('/').collect();
                    let valid = match parts.as_slice() {
                        [_] => true,
                        [_, t] => !t.is_empty(),
                        [_, _, normal] => !normal.is_empty(),
                        _ => false,
                    };
                    if !valid {
                        return Err(error("malformed face corner"));
                    }
                    for part in parts.iter().skip(1).filter(|p| !p.is_empty()) {
                        if part.parse::<i64>().ok().is_none_or(|v| v == 0) {
                            return Err(error("texture/normal index must be a nonzero integer"));
                        }
                    }
                    let index = parts[0]
                        .parse::<i64>()
                        .map_err(|_| error("invalid position index"))?;
                    let resolved = if index > 0 {
                        index - 1
                    } else {
                        ids.len() as i64 + index
                    };
                    if index == 0 || resolved < 0 {
                        return Err(error("position index out of range"));
                    }
                    face.push(
                        *ids.get(resolved as usize)
                            .ok_or_else(|| error("position index out of range"))?,
                    );
                    if face.len() >= 3 {
                        triangles = triangles
                            .checked_add(1)
                            .ok_or_else(|| error("triangle count overflow"))?;
                        if triangles > limits.max_triangles {
                            return Err(error("triangle limit exceeded"));
                        }
                    }
                }
                if face.len() < 3 {
                    return Err(error("face needs at least three corners"));
                }
                for k in 1..face.len() - 1 {
                    builder.triangle([face[0], face[k], face[k + 1]]);
                }
            }
            Some("vt" | "vn" | "vp" | "o" | "g" | "s" | "mtllib" | "usemtl" | "l" | "p") => {}
            Some(_) => return Err(error("unsupported OBJ statement")),
        }
        logical.clear();
    }
    if !logical.is_empty() {
        return Err(ReadError(format!("line {start}: unfinished continuation")));
    }
    finish(builder)
}
