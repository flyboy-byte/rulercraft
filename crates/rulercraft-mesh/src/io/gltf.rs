// SPDX-License-Identifier: MIT OR Apache-2.0
//! glTF 2.0 and GLB triangle import, written from the Khronos glTF 2.0 specification
//! (docs/specs/mesh-import.md "glTF and GLB"). Positions are metres by the standard.
use super::{ImportLimits, ReadError};
use crate::{Mesh, MeshBuilder, Unit};
use serde_json::Value;

const MAGIC: u32 = 0x4654_6C67; // "glTF"
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;
const FLOAT: u64 = 5126;

type Matrix = [f64; 16]; // column-major, as glTF stores it
const IDENTITY: Matrix = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

fn fail(why: impl Into<String>) -> ReadError {
    ReadError(why.into())
}

/// Reads a `.glb` (binary container) or `.gltf` (JSON with embedded buffers).
pub fn read_gltf(bytes: &[u8], binary: bool, limits: ImportLimits) -> Result<Mesh, ReadError> {
    super::check_bytes(bytes, limits)?;
    let (json, bin) = if binary {
        split_glb(bytes)?
    } else {
        (bytes, None)
    };
    let doc: Value = serde_json::from_slice(json)
        .map_err(|e| fail(format!("glTF JSON is not valid JSON: {e}")))?;
    let version = doc
        .pointer("/asset/version")
        .and_then(Value::as_str)
        .unwrap_or("");
    if version.split('.').next() != Some("2") {
        return Err(fail(format!(
            "glTF version `{version}` is not supported; only glTF 2.0 is read"
        )));
    }
    if let Some(req) = doc.get("extensionsRequired").and_then(Value::as_array)
        && !req.is_empty()
    {
        let names: Vec<&str> = req.iter().filter_map(Value::as_str).collect();
        return Err(fail(format!(
            "the file requires glTF extensions this reader does not support: {}; re-export without compression",
            names.join(", ")
        )));
    }
    let buffers = load_buffers(&doc, bin)?;
    let doc = Doc {
        json: &doc,
        buffers,
    };

    // Which mesh goes where: (mesh index, world matrix).
    let mut placed: Vec<(usize, Matrix)> = Vec::new();
    let nodes = doc.list("nodes");
    let scenes = doc.list("scenes");
    if scenes.is_empty() {
        for m in 0..doc.list("meshes").len() {
            placed.push((m, IDENTITY));
        }
    } else {
        let s = match doc.json.get("scene") {
            Some(v) => index(v, "scene")?,
            None => 0,
        };
        let scene = scenes
            .get(s)
            .ok_or_else(|| fail(format!("scene {s} does not exist")))?;
        let mut used = vec![false; nodes.len()];
        let mut stack: Vec<(usize, Matrix)> = Vec::new();
        for r in list_of(scene, "nodes") {
            stack.push((index(r, "scene node")?, IDENTITY));
        }
        while let Some((n, parent)) = stack.pop() {
            let node = nodes
                .get(n)
                .ok_or_else(|| fail(format!("node {n} does not exist")))?;
            if std::mem::replace(&mut used[n], true) {
                return Err(fail(format!(
                    "node {n} is used more than once; glTF nodes must form trees"
                )));
            }
            let world = mul(&parent, &local(node, n)?);
            if let Some(m) = node.get("mesh") {
                placed.push((index(m, "node mesh")?, world));
            }
            for c in list_of(node, "children") {
                stack.push((index(c, "child node")?, world));
            }
        }
    }

    // Count expanded geometry from accessor counts alone, before decoding anything.
    let meshes = doc.list("meshes");
    let mut instances = vec![0usize; meshes.len()];
    for (m, _) in &placed {
        *instances
            .get_mut(*m)
            .ok_or_else(|| fail(format!("mesh {m} does not exist")))? += 1;
    }
    let mut metas: Vec<Vec<Meta>> = Vec::with_capacity(meshes.len());
    let (mut tris, mut verts) = (0usize, 0usize);
    for (mi, mesh) in meshes.iter().enumerate() {
        let mut out = Vec::new();
        if instances[mi] > 0 {
            for p in list_of(mesh, "primitives") {
                if let Some(meta) = doc.meta(p, mi)? {
                    tris = tris.saturating_add(meta.triangles.saturating_mul(instances[mi]));
                    verts = verts.saturating_add(meta.vertices.saturating_mul(instances[mi]));
                    out.push(meta);
                }
            }
        }
        metas.push(out);
    }
    if tris > limits.max_triangles {
        return Err(fail(format!(
            "triangle limit exceeded: the scene expands to {tris} triangles (limit {})",
            limits.max_triangles
        )));
    }
    if verts > limits.max_vertices {
        return Err(fail(format!(
            "vertex limit exceeded: the scene expands to {verts} vertices (limit {})",
            limits.max_vertices
        )));
    }

    // Decode each accessor once, however many primitives use it.
    let mut positions: std::collections::HashMap<usize, Vec<[f64; 3]>> = Default::default();
    let mut orders: std::collections::HashMap<(usize, usize), Vec<u32>> = Default::default();
    use std::collections::hash_map::Entry;
    for meta in metas.iter().flatten() {
        if let Entry::Vacant(e) = positions.entry(meta.position) {
            e.insert(doc.positions(meta.position)?);
        }
        let n = positions[&meta.position].len();
        if let Some(i) = meta.indices
            && let Entry::Vacant(e) = orders.entry((i, n))
        {
            e.insert(doc.indices(i, n)?);
        }
    }
    let prims: Vec<Vec<Prim<'_>>> = metas
        .iter()
        .map(|ms| {
            ms.iter()
                .map(|m| {
                    let pos = &positions[&m.position];
                    Prim {
                        positions: pos,
                        order: m.indices.map(|i| orders[&(i, pos.len())].as_slice()),
                        mode: m.mode,
                    }
                })
                .collect()
        })
        .collect();

    let mut builder = MeshBuilder::new(Unit::M);
    for (m, world) in &placed {
        let flip = det3(world) < 0.0;
        for p in &prims[*m] {
            let mut ids = Vec::with_capacity(p.positions.len());
            for q in p.positions {
                let w = apply(world, *q);
                let f = w.map(|v| v as f32);
                let id = if f.iter().all(|v| v.is_finite()) {
                    super::vertex(&mut builder, f, Unit::M)
                } else {
                    None
                };
                ids.push(id.ok_or_else(|| {
                    fail(format!(
                        "mesh {m}: a transformed position is not a finite number in range"
                    ))
                })?);
            }
            for t in p.triangles() {
                let [a, b, c] = t.map(|i| ids[i as usize]);
                builder.triangle(if flip { [a, c, b] } else { [a, b, c] });
            }
        }
    }
    super::finish(builder)
}

/// Splits a GLB container into its JSON chunk and optional BIN chunk.
fn split_glb(bytes: &[u8]) -> Result<(&[u8], Option<&[u8]>), ReadError> {
    let word = |at: usize| -> Option<u32> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if word(0) != Some(MAGIC) {
        return Err(fail(
            "not a GLB file: the header does not start with `glTF`",
        ));
    }
    if word(4) != Some(2) {
        return Err(fail(format!(
            "GLB container version {} is not supported; only version 2 is read",
            word(4).unwrap_or(0)
        )));
    }
    let total = word(8).unwrap_or(0) as usize;
    if total != bytes.len() {
        return Err(fail(format!(
            "GLB header length {total} does not match the file's {} bytes; it is truncated or padded",
            bytes.len()
        )));
    }
    let mut at = 12;
    let (mut json, mut bin) = (None, None);
    while at < bytes.len() {
        let (len, kind) = match (word(at), word(at + 4)) {
            (Some(l), Some(k)) => (l as usize, k),
            _ => return Err(fail(format!("GLB chunk header at byte {at} is cut off"))),
        };
        let body = bytes.get(at + 8..at + 8 + len).ok_or_else(|| {
            fail(format!(
                "GLB chunk at byte {at} runs past the end of the file"
            ))
        })?;
        match kind {
            CHUNK_JSON if json.is_none() && at == 12 => json = Some(body),
            CHUNK_BIN if bin.is_none() && json.is_some() => bin = Some(body),
            _ if json.is_none() => return Err(fail("GLB's first chunk must be JSON")),
            _ => {} // unknown chunks are skipped, as the standard says
        }
        at += 8 + len;
    }
    Ok((json.ok_or_else(|| fail("GLB has no JSON chunk"))?, bin))
}

fn load_buffers<'a>(doc: &Value, bin: Option<&'a [u8]>) -> Result<Vec<Bytes<'a>>, ReadError> {
    let mut out = Vec::new();
    for (i, b) in doc
        .get("buffers")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
    {
        let len = usize_of(b.get("byteLength"), "buffer byteLength")?
            .ok_or_else(|| fail(format!("buffer {i} has no byteLength")))?;
        let data = match b.get("uri").and_then(Value::as_str) {
            None if i == 0 => {
                Bytes::Borrowed(bin.ok_or_else(|| {
                    fail("buffer 0 has no uri and the file has no GLB binary chunk")
                })?)
            }
            None => return Err(fail(format!("buffer {i} has no data"))),
            Some(uri) if uri.starts_with("data:") => {
                let (_, data) = uri
                    .split_once(";base64,")
                    .ok_or_else(|| fail(format!("buffer {i}: only base64 data URIs are read")))?;
                Bytes::Owned(
                    decode_base64(data)
                        .ok_or_else(|| fail(format!("buffer {i}: the base64 data is not valid")))?,
                )
            }
            Some(uri) => {
                return Err(fail(format!(
                    "buffer {i} is stored in a separate file (`{uri}`); export as .glb or embed the buffers"
                )));
            }
        };
        if data.get().len() < len {
            return Err(fail(format!(
                "buffer {i} holds {} bytes but declares {len}",
                data.get().len()
            )));
        }
        // The buffer is as long as it declares; container padding or extra bytes are not part of it.
        out.push(match data {
            Bytes::Borrowed(b) => Bytes::Borrowed(&b[..len]),
            Bytes::Owned(mut v) => {
                v.truncate(len);
                Bytes::Owned(v)
            }
        });
    }
    Ok(out)
}

enum Bytes<'a> {
    Borrowed(&'a [u8]),
    Owned(Vec<u8>),
}
impl Bytes<'_> {
    fn get(&self) -> &[u8] {
        match self {
            Bytes::Borrowed(b) => b,
            Bytes::Owned(v) => v,
        }
    }
}

struct Doc<'a> {
    json: &'a Value,
    buffers: Vec<Bytes<'a>>,
}

/// What a primitive needs, read from the JSON without touching buffers.
struct Meta {
    position: usize,
    indices: Option<usize>,
    mode: u64,
    vertices: usize,
    triangles: usize,
}

/// One primitive's local positions (metres) and its triangle list or strip/fan order.
struct Prim<'a> {
    positions: &'a [[f64; 3]],
    order: Option<&'a [u32]>,
    mode: u64,
}
impl Prim<'_> {
    fn triangles(&self) -> Vec<[u32; 3]> {
        let seq: Vec<u32>;
        let o = match self.order {
            Some(o) => o,
            None => {
                seq = (0..self.positions.len() as u32).collect();
                &seq
            }
        };
        match self.mode {
            4 => o.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
            5 => (0..o.len().saturating_sub(2))
                .map(|i| {
                    if i % 2 == 0 {
                        [o[i], o[i + 1], o[i + 2]]
                    } else {
                        [o[i + 1], o[i], o[i + 2]]
                    }
                })
                .collect(),
            _ => (0..o.len().saturating_sub(2))
                .map(|i| [o[0], o[i + 1], o[i + 2]])
                .collect(),
        }
    }
}

fn triangle_count(mode: u64, n: usize) -> usize {
    if mode == 4 {
        n / 3
    } else {
        n.saturating_sub(2)
    }
}

impl Doc<'_> {
    fn list(&self, key: &str) -> &[Value] {
        list_of(self.json, key)
    }

    /// A primitive's accessors and counts, from the JSON alone; `None` for points and lines.
    fn meta(&self, p: &Value, mesh: usize) -> Result<Option<Meta>, ReadError> {
        let mode = p.get("mode").and_then(Value::as_u64).unwrap_or(4);
        if !(4..=6).contains(&mode) {
            return Ok(None); // points and lines carry no surface
        }
        let pos = p
            .pointer("/attributes/POSITION")
            .ok_or_else(|| fail(format!("mesh {mesh}: a primitive has no POSITION")))?;
        let position = index(pos, "POSITION accessor")?;
        let indices = p
            .get("indices")
            .map(|v| index(v, "indices accessor"))
            .transpose()?;
        let count = |a: usize| -> Result<usize, ReadError> {
            let acc = self
                .list("accessors")
                .get(a)
                .ok_or_else(|| fail(format!("accessor {a} does not exist")))?;
            Ok(usize_of(acc.get("count"), "accessor count")?.unwrap_or(0))
        };
        let vertices = count(position)?;
        let order = match indices {
            Some(i) => count(i)?,
            None => vertices,
        };
        Ok(Some(Meta {
            position,
            indices,
            mode,
            vertices,
            triangles: triangle_count(mode, order),
        }))
    }

    /// The bytes of accessor `a`'s elements: (data, first offset, stride, count, component type).
    fn accessor(
        &self,
        a: usize,
        kind: &str,
        elem: impl Fn(u64) -> Option<usize>,
    ) -> Result<(&[u8], usize, usize, usize, u64), ReadError> {
        let acc = self
            .list("accessors")
            .get(a)
            .ok_or_else(|| fail(format!("accessor {a} does not exist")))?;
        if acc.get("sparse").is_some() {
            return Err(fail(format!(
                "accessor {a} is sparse; sparse data is not read"
            )));
        }
        let ty = acc.get("type").and_then(Value::as_str).unwrap_or("");
        if ty != kind {
            return Err(fail(format!("accessor {a} is {ty}, expected {kind}")));
        }
        let ct = acc
            .get("componentType")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let size = elem(ct).ok_or_else(|| {
            fail(if kind == "VEC3" {
                format!("accessor {a}: positions must be float (component type 5126), found {ct}")
            } else {
                format!("accessor {a}: indices must be unsigned byte, short or int, found {ct}")
            })
        })?;
        let count = usize_of(acc.get("count"), "accessor count")?.unwrap_or(0);
        let off = usize_of(acc.get("byteOffset"), "accessor byteOffset")?.unwrap_or(0);
        let v = index(
            acc.get("bufferView")
                .ok_or_else(|| fail(format!("accessor {a} has no bufferView")))?,
            "bufferView",
        )?;
        let view = self
            .list("bufferViews")
            .get(v)
            .ok_or_else(|| fail(format!("buffer view {v} does not exist")))?;
        let b = index(
            view.get("buffer")
                .ok_or_else(|| fail(format!("buffer view {v} has no buffer")))?,
            "buffer",
        )?;
        let buf = self
            .buffers
            .get(b)
            .ok_or_else(|| fail(format!("buffer {b} does not exist")))?
            .get();
        let vo = usize_of(view.get("byteOffset"), "view byteOffset")?.unwrap_or(0);
        let vl = usize_of(view.get("byteLength"), "view byteLength")?
            .ok_or_else(|| fail(format!("buffer view {v} has no byteLength")))?;
        let stride = usize_of(view.get("byteStride"), "byteStride")?.unwrap_or(size);
        if stride < size {
            return Err(fail(format!(
                "buffer view {v}: byteStride {stride} is smaller than an element"
            )));
        }
        let view_bytes = vo
            .checked_add(vl)
            .and_then(|end| buf.get(vo..end))
            .ok_or_else(|| fail(format!("buffer view {v} is out of range of buffer {b}")))?;
        if count > 0 {
            let last = (count - 1)
                .checked_mul(stride)
                .and_then(|x| x.checked_add(off))
                .and_then(|x| x.checked_add(size));
            if last.is_none_or(|end| end > view_bytes.len()) {
                return Err(fail(format!(
                    "accessor {a} reads out of range of buffer view {v}"
                )));
            }
        }
        Ok((view_bytes, off, stride, count, ct))
    }

    fn positions(&self, a: usize) -> Result<Vec<[f64; 3]>, ReadError> {
        let (bytes, off, stride, count, _) =
            self.accessor(a, "VEC3", |ct| (ct == FLOAT).then_some(12))?;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let at = off + i * stride;
            let mut p = [0f64; 3];
            for (k, slot) in p.iter_mut().enumerate() {
                let b = &bytes[at + 4 * k..at + 4 * k + 4];
                let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                if !v.is_finite() {
                    return Err(fail(format!(
                        "accessor {a}: position {i} is not a finite number"
                    )));
                }
                *slot = f64::from(v);
            }
            out.push(p);
        }
        Ok(out)
    }

    fn indices(&self, a: usize, positions: usize) -> Result<Vec<u32>, ReadError> {
        let (bytes, off, stride, count, ct) = self.accessor(a, "SCALAR", |ct| match ct {
            5121 => Some(1),
            5123 => Some(2),
            5125 => Some(4),
            _ => None,
        })?;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let at = off + i * stride;
            let v = match ct {
                5121 => u32::from(bytes[at]),
                5123 => u32::from(u16::from_le_bytes([bytes[at], bytes[at + 1]])),
                _ => u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]),
            };
            if v as usize >= positions {
                return Err(fail(format!(
                    "accessor {a}: index {v} is past the {positions} positions"
                )));
            }
            out.push(v);
        }
        Ok(out)
    }
}

fn list_of<'v>(v: &'v Value, key: &str) -> &'v [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

fn index(v: &Value, what: &str) -> Result<usize, ReadError> {
    v.as_u64()
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| fail(format!("{what} must be a nonnegative integer")))
}

fn usize_of(v: Option<&Value>, what: &str) -> Result<Option<usize>, ReadError> {
    v.map(|x| index(x, what)).transpose()
}

fn numbers<const N: usize>(
    node: &Value,
    key: &str,
    n: usize,
) -> Result<Option<[f64; N]>, ReadError> {
    let Some(v) = node.get(key) else {
        return Ok(None);
    };
    let a = v
        .as_array()
        .filter(|a| a.len() == N)
        .ok_or_else(|| fail(format!("node {n}: {key} needs {N} numbers")))?;
    let mut out = [0f64; N];
    for (slot, x) in out.iter_mut().zip(a) {
        *slot = x.as_f64().filter(|f| f.is_finite()).ok_or_else(|| {
            fail(format!(
                "node {n}: {key} has a value that is not a finite number"
            ))
        })?;
    }
    Ok(Some(out))
}

/// A node's local transform: its matrix, or translation · rotation · scale.
fn local(node: &Value, n: usize) -> Result<Matrix, ReadError> {
    if let Some(m) = numbers::<16>(node, "matrix", n)? {
        return Ok(m);
    }
    let t = numbers::<3>(node, "translation", n)?.unwrap_or([0.0; 3]);
    let [x, y, z, w] = numbers::<4>(node, "rotation", n)?.unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let s = numbers::<3>(node, "scale", n)?.unwrap_or([1.0; 3]);
    let q = (x * x + y * y + z * z + w * w).sqrt();
    if q == 0.0 {
        return Err(fail(format!("node {n}: rotation is a zero quaternion")));
    }
    let (x, y, z, w) = (x / q, y / q, z / q, w / q);
    // Rotation matrix columns (column-major), each scaled by its axis scale.
    let r = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + z * w),
            2.0 * (x * z - y * w),
        ],
        [
            2.0 * (x * y - z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + x * w),
        ],
        [
            2.0 * (x * z + y * w),
            2.0 * (y * z - x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    let mut m = IDENTITY;
    for c in 0..3 {
        for row in 0..3 {
            m[c * 4 + row] = r[c][row] * s[c];
        }
    }
    m[12..15].copy_from_slice(&t);
    Ok(m)
}

fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = [0f64; 16];
    for c in 0..4 {
        for r in 0..4 {
            out[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    out
}

fn apply(m: &Matrix, p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
}

fn det3(m: &Matrix) -> f64 {
    m[0] * (m[5] * m[10] - m[9] * m[6]) - m[4] * (m[1] * m[10] - m[9] * m[2])
        + m[8] * (m[1] * m[6] - m[5] * m[2])
}

fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let s = s.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for chunk in s.chunks(4) {
        if chunk.len() == 1 {
            return None;
        }
        let mut n = 0u32;
        for (k, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * k);
        }
        let bytes = n.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    Some(out)
}
