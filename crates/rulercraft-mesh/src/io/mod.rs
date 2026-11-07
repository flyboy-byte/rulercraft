// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded mesh import selected by file extension.
pub mod obj;
pub mod ply;

use crate::{Mesh, MeshBuilder, Unit};
use std::fmt;

/// Work limits for mesh import.
#[derive(Debug, Clone, Copy)]
pub struct ImportLimits {
    /// Largest input in bytes.
    pub max_bytes: usize,
    /// Most source vertices (before merging).
    pub max_vertices: usize,
    /// Most candidate triangles (including degenerates).
    pub max_triangles: usize,
}
impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_bytes: 2 << 30,
            max_vertices: 50_000_000,
            max_triangles: 50_000_000,
        }
    }
}
/// An actionable import refusal, with a line or byte offset for malformed data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError(pub(crate) String);
impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ReadError {}

/// The supported reader name for a case-insensitive filename extension.
pub fn format_name(name: &str) -> Result<&'static str, ReadError> {
    match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "stl" => Ok("stl"),
        "ply" => Ok("ply"),
        "obj" => Ok("obj"),
        _ => Err(ReadError(
            "unknown extension: supported extensions are .stl, .ply, .obj".into(),
        )),
    }
}
/// Reads a mesh from bytes, converting file units to millimetres.
pub fn read_mesh(
    name: &str,
    bytes: &[u8],
    unit: Unit,
    limits: ImportLimits,
) -> Result<Mesh, ReadError> {
    let format = format_name(name)?;
    check_bytes(bytes, limits)?;
    match format {
        "ply" => ply::read_ply(bytes, unit, limits),
        "obj" => obj::read_obj(bytes, unit, limits),
        _ => {
            let m = crate::read_stl(
                bytes,
                unit,
                crate::Limits {
                    max_bytes: limits.max_bytes,
                    max_triangles: limits.max_triangles,
                },
            )
            .map_err(|e| ReadError(e.to_string()))?;
            if m.positions.len() > limits.max_vertices {
                return Err(ReadError("vertex limit exceeded".into()));
            }
            Ok(m)
        }
    }
}
fn check_bytes(bytes: &[u8], limits: ImportLimits) -> Result<(), ReadError> {
    if bytes.len() > limits.max_bytes {
        return Err(ReadError("input byte limit exceeded".into()));
    }
    if bytes.is_empty() {
        return Err(ReadError("the file contains no triangles".into()));
    }
    Ok(())
}
fn vertex(builder: &mut MeshBuilder, p: [f32; 3], unit: Unit) -> Option<u32> {
    if p.iter().any(|v| {
        !v.is_finite()
            || !(f64::from(*v) * unit.to_mm()).is_finite()
            || (f64::from(*v) * unit.to_mm()).abs() > f64::from(f32::MAX)
    }) {
        return None;
    }
    builder.vertex(p)
}
fn finish(builder: MeshBuilder) -> Result<Mesh, ReadError> {
    let mesh = builder.finish();
    if mesh.triangles.is_empty() {
        return Err(ReadError("the file contains no triangles".into()));
    }
    Ok(mesh)
}
