// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded mesh import selected by file extension.
pub mod gltf;
pub mod obj;
pub mod ply;
pub mod points;
pub mod threemf;
mod xml;

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
        .rsplit_once('.')
        .map_or("", |(_, extension)| extension)
        .to_ascii_lowercase()
        .as_str()
    {
        "stl" => Ok("stl"),
        "ply" => Ok("ply"),
        "obj" => Ok("obj"),
        "3mf" => Ok("3mf"),
        "gltf" => Ok("gltf"),
        "glb" => Ok("glb"),
        "xyz" | "pts" | "asc" => Err(ReadError(
            "a point cloud has no triangles: `fit` and `measure` read it; this needs a mesh (.stl, .ply, .obj, \
             .3mf, .gltf, .glb)"
                .into(),
        )),
        _ => Err(ReadError(
            "unknown extension: supported extensions are .stl, .ply, .obj, .3mf, .gltf, .glb, and for points \
             .xyz, .pts, .asc"
                .into(),
        )),
    }
}

/// Whether a filename names a text point cloud (.xyz, .pts, .asc), case-insensitively.
pub fn is_point_cloud_name(name: &str) -> bool {
    matches!(
        name.rsplit_once('.')
            .map_or("", |(_, extension)| extension)
            .to_ascii_lowercase()
            .as_str(),
        "xyz" | "pts" | "asc"
    )
}

/// Reads points to fit: a text point cloud or a PLY file's vertices (faces or not) as a mesh with no triangles,
/// and any other mesh format as a mesh.
pub fn read_cloud(
    name: &str,
    bytes: &[u8],
    unit: Unit,
    limits: ImportLimits,
) -> Result<Mesh, ReadError> {
    if is_point_cloud_name(name) {
        return points::read_text_points(bytes, unit, limits);
    }
    match format_name(name)? {
        "ply" => ply::read_ply_points(bytes, unit, limits),
        _ => read_mesh(name, bytes, unit, limits),
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
        "3mf" => threemf::read_threemf(bytes, limits),
        "obj" => obj::read_obj(bytes, unit, limits),
        "gltf" => gltf::read_gltf(bytes, false, limits),
        "glb" => gltf::read_gltf(bytes, true, limits),
        _ => {
            let m = crate::stl::read_stl_with_vertex_limit(
                bytes,
                unit,
                crate::Limits {
                    max_bytes: limits.max_bytes,
                    max_triangles: limits.max_triangles,
                },
                limits.max_vertices,
            )
            .map_err(|e| ReadError(e.to_string()))?;
            Ok(m)
        }
    }
}
pub(crate) fn check_bytes(bytes: &[u8], limits: ImportLimits) -> Result<(), ReadError> {
    if bytes.len() > limits.max_bytes {
        return Err(ReadError("input byte limit exceeded".into()));
    }
    if bytes.is_empty() {
        return Err(ReadError("the file contains no triangles".into()));
    }
    Ok(())
}
pub(crate) fn vertex(builder: &mut MeshBuilder, p: [f32; 3], unit: Unit) -> Option<u32> {
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
