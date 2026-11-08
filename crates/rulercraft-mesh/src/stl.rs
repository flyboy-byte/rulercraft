// SPDX-License-Identifier: MIT OR Apache-2.0
//! STL reading, ASCII and binary (docs/specs/mesh-import.md).
//!
//! Binary layout: an 80-byte header, a little-endian u32 triangle count, then 50 bytes per triangle
//! (normal, three corners, each three f32, and a u16 attribute). ASCII: `solid`, then
//! `facet normal ... / outer loop / vertex x y z ... / endloop / endfacet`, then `endsolid`.

use crate::mesh::{Mesh, MeshBuilder, Unit};
use std::fmt;

/// Size limits a read refuses to go past.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Largest input accepted, bytes.
    pub max_bytes: usize,
    /// Most candidate triangles accepted, including degenerates.
    pub max_triangles: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 2 << 30,
            max_triangles: 50_000_000,
        }
    }
}

/// Why a mesh could not be read. `Display` gives a message a user can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The input holds no triangles.
    Empty,
    /// The input is larger than the limit.
    TooLarge {
        /// Bytes given.
        bytes: usize,
        /// The limit.
        limit: usize,
    },
    /// More triangles than the limit.
    TooManyTriangles {
        /// Triangles found or claimed.
        triangles: usize,
        /// The limit.
        limit: usize,
    },
    /// The merged vertex limit was exceeded.
    TooManyVertices {
        /// The maximum allowed vertices.
        limit: usize,
    },
    /// A binary file whose length does not match its triangle count.
    BinaryLength {
        /// Triangles the header claims.
        claimed: u32,
        /// Bytes actually present.
        bytes: usize,
    },
    /// A coordinate is NaN or infinite.
    NotFinite {
        /// Triangle number (from 1) holding it.
        triangle: usize,
    },
    /// ASCII text that does not follow the format.
    Syntax {
        /// Line number (from 1).
        line: usize,
        /// What was wrong.
        problem: String,
    },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Empty => write!(f, "the file contains no triangles"),
            ReadError::TooLarge { bytes, limit } => {
                write!(f, "the file is {bytes} bytes, over the {limit}-byte limit")
            }
            ReadError::TooManyTriangles { triangles, limit } => {
                write!(
                    f,
                    "the file has {triangles} triangles, over the {limit}-triangle limit"
                )
            }
            ReadError::TooManyVertices { limit } => write!(f, "vertex limit exceeded ({limit})"),
            ReadError::BinaryLength { claimed, bytes } => write!(
                f,
                "binary STL header claims {claimed} triangles ({} bytes) but the file is {bytes} bytes; it is truncated or not an STL",
                84 + 50 * u64::from(*claimed)
            ),
            ReadError::NotFinite { triangle } => {
                write!(
                    f,
                    "triangle {triangle} has a coordinate that is not a finite number"
                )
            }
            ReadError::Syntax { line, problem } => write!(f, "line {line}: {problem}"),
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads an STL file given as bytes, in `unit`, into a millimetre mesh.
pub fn read_stl(bytes: &[u8], unit: Unit, limits: Limits) -> Result<Mesh, ReadError> {
    read_stl_with_vertex_limit(bytes, unit, limits, usize::MAX)
}

pub(crate) fn read_stl_with_vertex_limit(
    bytes: &[u8],
    unit: Unit,
    limits: Limits,
    max_vertices: usize,
) -> Result<Mesh, ReadError> {
    if bytes.len() > limits.max_bytes {
        return Err(ReadError::TooLarge {
            bytes: bytes.len(),
            limit: limits.max_bytes,
        });
    }
    let mesh = if binary_length_matches(bytes) || !looks_ascii(bytes) {
        read_binary(bytes, unit, limits, max_vertices)?
    } else {
        read_ascii(bytes, unit, limits, max_vertices)?
    };
    if mesh.triangles.is_empty() {
        return Err(ReadError::Empty);
    }
    Ok(mesh)
}

fn binary_length_matches(bytes: &[u8]) -> bool {
    bytes.len() >= 84 && {
        let n = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
        84 + 50 * u64::from(n) == bytes.len() as u64
    }
}

fn looks_ascii(bytes: &[u8]) -> bool {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes[start..].starts_with(b"solid")
}

fn read_binary(
    bytes: &[u8],
    unit: Unit,
    limits: Limits,
    max_vertices: usize,
) -> Result<Mesh, ReadError> {
    if bytes.len() < 84 {
        return Err(if bytes.is_empty() {
            ReadError::Empty
        } else {
            ReadError::BinaryLength {
                claimed: 0,
                bytes: bytes.len(),
            }
        });
    }
    let claimed = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
    if !binary_length_matches(bytes) {
        return Err(ReadError::BinaryLength {
            claimed,
            bytes: bytes.len(),
        });
    }
    if claimed as usize > limits.max_triangles {
        return Err(ReadError::TooManyTriangles {
            triangles: claimed as usize,
            limit: limits.max_triangles,
        });
    }
    let mut b = MeshBuilder::new(unit);
    for (i, rec) in bytes[84..].chunks_exact(50).enumerate() {
        let f = |o: usize| f32::from_le_bytes([rec[o], rec[o + 1], rec[o + 2], rec[o + 3]]);
        let mut t = [0u32; 3];
        for (c, slot) in t.iter_mut().enumerate() {
            let o = 12 + 12 * c;
            *slot = b
                .vertex([f(o), f(o + 4), f(o + 8)])
                .ok_or(ReadError::NotFinite { triangle: i + 1 })?;
            if *slot as usize >= max_vertices {
                return Err(ReadError::TooManyVertices {
                    limit: max_vertices,
                });
            }
        }
        b.triangle(t);
    }
    Ok(b.finish())
}

fn read_ascii(
    bytes: &[u8],
    unit: Unit,
    limits: Limits,
    max_vertices: usize,
) -> Result<Mesh, ReadError> {
    let text = String::from_utf8_lossy(bytes);
    let mut b = MeshBuilder::new(unit);
    let mut corners: Vec<u32> = Vec::new();
    let mut in_loop = false;
    let mut facets = 0usize;
    let mut candidates = 0usize;
    for (n, line) in text.lines().enumerate() {
        let line_no = n + 1;
        let mut words = line.split_whitespace();
        let Some(word) = words.next() else { continue };
        let syntax = |problem: &str| ReadError::Syntax {
            line: line_no,
            problem: problem.to_string(),
        };
        match word {
            "solid" | "endsolid" | "facet" | "endfacet" => {}
            "outer" => {
                if in_loop {
                    return Err(syntax("`outer loop` inside another loop"));
                }
                in_loop = true;
                corners.clear();
            }
            "vertex" => {
                if !in_loop {
                    return Err(syntax("`vertex` outside `outer loop`"));
                }
                let mut p = [0f32; 3];
                for v in &mut p {
                    *v = words
                        .next()
                        .and_then(|w| w.parse::<f32>().ok())
                        .ok_or_else(|| syntax("`vertex` needs three numbers"))?;
                }
                if corners.len() >= 2 {
                    candidates = candidates.saturating_add(1);
                    if candidates > limits.max_triangles {
                        return Err(ReadError::TooManyTriangles {
                            triangles: candidates,
                            limit: limits.max_triangles,
                        });
                    }
                }
                facets += usize::from(corners.is_empty());
                let id = b
                    .vertex(p)
                    .ok_or(ReadError::NotFinite { triangle: facets })?;
                if id as usize >= max_vertices {
                    return Err(ReadError::TooManyVertices {
                        limit: max_vertices,
                    });
                }
                corners.push(id);
            }
            "endloop" => {
                if !in_loop || corners.len() < 3 {
                    return Err(syntax("a loop needs at least three vertices"));
                }
                in_loop = false;
                for k in 1..corners.len() - 1 {
                    b.triangle([corners[0], corners[k], corners[k + 1]]);
                }
            }
            other => return Err(syntax(&format!("unexpected `{other}`"))),
        }
    }
    if in_loop {
        return Err(ReadError::Syntax {
            line: text.lines().count(),
            problem: "the file ends inside a loop".into(),
        });
    }
    Ok(b.finish())
}
