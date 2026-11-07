// SPDX-License-Identifier: MIT OR Apache-2.0
//! Geometry-only 3MF import from the core format description, not an archive extractor.
use super::xml::{self, Node};
use super::{ImportLimits, ReadError};
use crate::{Mesh, MeshBuilder, Unit};
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};

const CORE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const START: &str = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel";
type Matrix = [[f64; 3]; 4];
const IDENTITY: Matrix = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0., 0., 0.]];
struct Instance {
    id: u32,
    matrix: Matrix,
    offset: usize,
}
enum Geometry {
    Mesh(Vec<[f64; 3]>, Vec<[usize; 3]>),
    Components(Vec<Instance>),
}
struct Object {
    geometry: Geometry,
    offset: usize,
}
fn fail(at: usize, why: impl std::fmt::Display) -> ReadError {
    ReadError(format!("model XML byte offset {at}: {why}"))
}
fn uint(node: &Node, key: &str) -> Result<u32, ReadError> {
    node.attr(key)?
        .trim()
        .parse()
        .map_err(|_| node.error(format!("{key} must be an unsigned integer")))
}
fn scalar(text: &str, node: &Node) -> Result<f64, ReadError> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| node.error("expected finite number"))
}
fn instance(node: &Node) -> Result<Instance, ReadError> {
    if node
        .attrs
        .keys()
        .any(|a| a == "path" || a.ends_with(":path"))
    {
        return Err(node.error("cross-part component paths are unsupported"));
    }
    let mut matrix = IDENTITY;
    if let Some(text) = node.attrs.get("transform") {
        let mut values = text.split_whitespace();
        for slot in matrix.iter_mut().flatten() {
            *slot = scalar(
                values
                    .next()
                    .ok_or_else(|| node.error("transform needs 12 numbers"))?,
                node,
            )?;
        }
        if values.next().is_some() {
            return Err(node.error("transform needs 12 numbers"));
        }
    }
    Ok(Instance {
        id: uint(node, "objectid")?,
        matrix,
        offset: node.offset,
    })
}
fn apply(p: [f64; 3], m: Matrix) -> [f64; 3] {
    [0, 1, 2].map(|j| p[0] * m[0][j] + p[1] * m[1][j] + p[2] * m[2][j] + m[3][j])
}
fn compose(child: Matrix, parent: Matrix) -> Matrix {
    let mut out = [[0.; 3]; 4];
    for i in 0..4 {
        for j in 0..3 {
            out[i][j] = (0..3).map(|k| child[i][k] * parent[k][j]).sum::<f64>()
                + if i == 3 { parent[3][j] } else { 0. };
        }
    }
    out
}
fn determinant(m: Matrix) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}
fn entry(
    zip: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
    remaining: &mut usize,
) -> Result<Vec<u8>, ReadError> {
    let mut file = zip
        .by_name(name)
        .map_err(|e| ReadError(format!("ZIP part {name}: {e}")))?;
    if file.size() > *remaining as u64 {
        return Err(ReadError(format!(
            "ZIP part {name}: uncompressed byte limit exceeded"
        )));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take((*remaining as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| ReadError(format!("ZIP part {name}: {e}")))?;
    if bytes.len() > *remaining {
        return Err(ReadError(format!(
            "ZIP part {name}: uncompressed byte limit exceeded"
        )));
    }
    *remaining -= bytes.len();
    Ok(bytes)
}
struct Expansion<'a> {
    objects: &'a BTreeMap<u32, Object>,
    builder: MeshBuilder,
    limits: ImportLimits,
    scale: f64,
    vertices: usize,
    triangles: usize,
    instances: usize,
}
impl Expansion<'_> {
    fn emit(
        &mut self,
        item: &Instance,
        parent: Matrix,
        path: &mut HashSet<u32>,
    ) -> Result<(), ReadError> {
        self.instances += 1;
        if self.instances > 100_000 || path.len() >= 64 {
            return Err(fail(item.offset, "component instance/depth limit exceeded"));
        }
        if !path.insert(item.id) {
            return Err(fail(item.offset, "component cycle"));
        }
        let object = self
            .objects
            .get(&item.id)
            .ok_or_else(|| fail(item.offset, "missing object reference"))?;
        let matrix = compose(item.matrix, parent);
        if matrix.iter().flatten().any(|v| !v.is_finite()) {
            return Err(fail(item.offset, "transform overflow"));
        }
        match &object.geometry {
            Geometry::Mesh(vertices, triangles) => {
                self.vertices = self.vertices.saturating_add(vertices.len());
                self.triangles = self.triangles.saturating_add(triangles.len());
                if self.vertices > self.limits.max_vertices
                    || self.triangles > self.limits.max_triangles
                {
                    return Err(fail(item.offset, "expanded vertex/triangle limit exceeded"));
                }
                let mut indices = Vec::with_capacity(vertices.len());
                for &p in vertices {
                    let p = apply(p, matrix).map(|v| v * self.scale);
                    if p.iter()
                        .any(|v| !v.is_finite() || v.abs() > f64::from(f32::MAX))
                    {
                        return Err(fail(
                            object.offset,
                            "transformed coordinate overflows f32 millimetres",
                        ));
                    }
                    indices.push(
                        self.builder
                            .vertex(p.map(|v| v as f32))
                            .ok_or_else(|| fail(object.offset, "invalid vertex"))?,
                    );
                }
                let reverse = determinant(matrix) < 0.;
                for t in triangles {
                    let mut t = t.map(|i| indices[i]);
                    if reverse {
                        t.swap(1, 2);
                    }
                    self.builder.triangle(t);
                }
            }
            Geometry::Components(items) => {
                for child in items {
                    self.emit(child, matrix, path)?;
                }
            }
        }
        path.remove(&item.id);
        Ok(())
    }
}
/// Reads the primary core model and flattens its build into millimetre triangles.
pub fn read_threemf(bytes: &[u8], limits: ImportLimits) -> Result<Mesh, ReadError> {
    super::check_bytes(bytes, limits)?;
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| ReadError(format!("ZIP byte offset 0: {e}")))?;
    if zip.len() > 10_000 {
        return Err(ReadError("ZIP entry limit exceeded".into()));
    }
    let mut remaining = limits.max_bytes;
    let rels = xml::parse(&entry(&mut zip, "_rels/.rels", &mut remaining)?, 100_000)?;
    if rels.name != "Relationships" || rels.namespace != RELS {
        return Err(rels.error("expected package Relationships"));
    }
    let mut starts = rels
        .children("Relationship")
        .filter(|n| n.attrs.get("Type").is_some_and(|v| v == START));
    let start = starts
        .next()
        .ok_or_else(|| rels.error("missing start-part relationship"))?;
    if starts.next().is_some() {
        return Err(rels.error("ambiguous start-part relationships"));
    }
    if start
        .attrs
        .get("TargetMode")
        .is_some_and(|v| v != "Internal")
    {
        return Err(start.error("external start-part target unsupported"));
    }
    let name = start.attr("Target")?.trim_start_matches('/');
    if name.is_empty()
        || name.contains(['\\', '%', ':', '?', '#'])
        || name
            .split('/')
            .any(|p| p.is_empty() || p == ".." || p == ".")
    {
        return Err(start.error("unsupported start-part path"));
    }
    let xml = entry(&mut zip, name, &mut remaining)?;
    let root = xml::parse(
        &xml,
        limits
            .max_vertices
            .saturating_add(limits.max_triangles)
            .saturating_add(500_000),
    )?;
    if root.name != "model" || root.namespace != CORE {
        return Err(root.error("expected 3MF core model namespace"));
    }
    if root
        .attrs
        .get("requiredextensions")
        .is_some_and(|v| !v.trim().is_empty())
    {
        return Err(root.error("required extensions are unsupported"));
    }
    let scale = match root
        .attrs
        .get("unit")
        .map(|s| s.trim())
        .unwrap_or("millimeter")
    {
        "micron" => 0.001,
        "millimeter" => 1.,
        "centimeter" => 10.,
        "inch" => 25.4,
        "foot" => 304.8,
        "meter" => 1000.,
        _ => return Err(root.error("unknown model unit")),
    };
    let mut objects = BTreeMap::new();
    let mut resource_ids = HashSet::new();
    let (mut vertex_count, mut triangle_count) = (0usize, 0usize);
    let resources = root.one("resources")?;
    let mut declared_instances = 0usize;
    for resource in resources.children.iter().filter(|n| n.namespace == CORE) {
        let id = uint(resource, "id")?;
        if id == 0 || !resource_ids.insert(id) {
            return Err(resource.error("resource ids must be positive and unique"));
        }
        if resource_ids.len() > 100_000 {
            return Err(resource.error("resource limit exceeded"));
        }
        if resource.name != "object" {
            continue;
        }
        let mesh = resource.children("mesh").next();
        let components = resource.children("components").next();
        let geometry = match (mesh, components) {
            (Some(mesh), None) => {
                // one() also refuses duplicate geometry containers.
                resource.one("mesh")?;
                let mut vertices = Vec::new();
                let mut triangles = Vec::new();
                for v in mesh.one("vertices")?.children("vertex") {
                    vertex_count += 1;
                    if vertex_count > limits.max_vertices {
                        return Err(v.error("source vertex limit exceeded"));
                    }
                    vertices.push([
                        scalar(v.attr("x")?, v)?,
                        scalar(v.attr("y")?, v)?,
                        scalar(v.attr("z")?, v)?,
                    ]);
                }
                for t in mesh.one("triangles")?.children("triangle") {
                    triangle_count += 1;
                    if triangle_count > limits.max_triangles {
                        return Err(t.error("source triangle limit exceeded"));
                    }
                    let indices = [
                        uint(t, "v1")? as usize,
                        uint(t, "v2")? as usize,
                        uint(t, "v3")? as usize,
                    ];
                    if indices.iter().any(|i| *i >= vertices.len()) {
                        return Err(t.error("triangle vertex index out of range"));
                    }
                    triangles.push(indices);
                }
                Geometry::Mesh(vertices, triangles)
            }
            (None, Some(_)) => {
                let mut items = Vec::new();
                for node in resource.one("components")?.children("component") {
                    declared_instances += 1;
                    if declared_instances > 100_000 {
                        return Err(node.error("declared component limit exceeded"));
                    }
                    items.push(instance(node)?);
                }
                Geometry::Components(items)
            }
            _ => return Err(resource.error("object needs exactly one mesh or components")),
        };
        objects.insert(
            id,
            Object {
                geometry,
                offset: resource.offset,
            },
        );
    }
    // Validate even unused component graphs without expanding their geometry.
    fn visit(
        id: u32,
        objects: &BTreeMap<u32, Object>,
        active: &mut HashSet<u32>,
        done: &mut HashSet<u32>,
    ) -> Result<(), ReadError> {
        if done.contains(&id) {
            return Ok(());
        }
        let object = objects
            .get(&id)
            .ok_or_else(|| fail(0, "missing object reference"))?;
        if active.len() >= 64 {
            return Err(fail(object.offset, "component depth limit exceeded"));
        }
        if !active.insert(id) {
            return Err(fail(object.offset, "component cycle"));
        }
        if let Geometry::Components(items) = &object.geometry {
            for item in items {
                visit(item.id, objects, active, done)?;
            }
        }
        active.remove(&id);
        done.insert(id);
        Ok(())
    }
    let mut done = HashSet::new();
    for &id in objects.keys() {
        visit(id, &objects, &mut HashSet::new(), &mut done)?;
    }
    let mut out = Expansion {
        objects: &objects,
        builder: MeshBuilder::new(Unit::Mm),
        limits,
        scale,
        vertices: 0,
        triangles: 0,
        instances: 0,
    };
    for node in root.one("build")?.children("item") {
        let item = instance(node)?;
        if !objects.contains_key(&item.id) {
            return Err(node.error("missing build object reference"));
        }
        match node
            .attrs
            .get("printable")
            .map(String::as_str)
            .unwrap_or("true")
        {
            "false" | "0" => continue,
            "true" | "1" => (),
            _ => return Err(node.error("printable must be true, false, 1 or 0")),
        }
        out.emit(&item, IDENTITY, &mut HashSet::new())?;
    }
    super::finish(out.builder)
}
