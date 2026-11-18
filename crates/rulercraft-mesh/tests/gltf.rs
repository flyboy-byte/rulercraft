// SPDX-License-Identifier: MIT OR Apache-2.0
//! glTF 2.0 / GLB import (docs/specs/mesh-import.md "glTF and GLB"). Known answers: a cube 1/64 m on a
//! side (exactly 15.625 mm), transforms with hand-computed results, and the same cube read as STL.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::io::{ImportLimits, read_mesh};
use rulercraft_mesh::{Mesh, Unit};
use serde_json::{Value, json};

const S: f32 = 1.0 / 64.0;
const MM: f64 = 15.625;

fn cube() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let p = (0..8)
        .map(|i| {
            [
                (i & 1) as f32 * S,
                ((i >> 1) & 1) as f32 * S,
                ((i >> 2) & 1) as f32 * S,
            ]
        })
        .collect();
    // Outward winding (right-hand normals point away from the centre).
    let t = vec![
        [0, 2, 1],
        [1, 2, 3], // z = 0
        [4, 5, 6],
        [5, 7, 6], // z = 1
        [0, 1, 4],
        [1, 5, 4], // y = 0
        [2, 6, 3],
        [3, 6, 7], // y = 1
        [0, 4, 2],
        [2, 4, 6], // x = 0
        [1, 3, 5],
        [3, 7, 5], // x = 1
    ];
    (p, t)
}

fn f32s(v: &[[f32; 3]]) -> Vec<u8> {
    v.iter().flatten().flat_map(|x| x.to_le_bytes()).collect()
}

/// A one-mesh document: buffer 0 holds positions then indices (u32 unless `index_type` says otherwise).
fn doc(prims: Value, nodes: Value, bin_len: usize) -> Value {
    json!({
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": nodes,
        "meshes": [{"primitives": prims}],
        "buffers": [{"byteLength": bin_len}],
    })
}

/// Cube document with positions at view 0 and indices at view 1, plus the binary buffer.
fn cube_doc(index_type: u32, nodes: Value) -> (Value, Vec<u8>) {
    let (p, t) = cube();
    let mut bin = f32s(&p);
    let pos_len = bin.len();
    let flat: Vec<u32> = t.iter().flatten().copied().collect();
    match index_type {
        5121 => bin.extend(flat.iter().map(|&i| i as u8)),
        5123 => bin.extend(flat.iter().flat_map(|&i| (i as u16).to_le_bytes())),
        _ => bin.extend(flat.iter().flat_map(|&i| i.to_le_bytes())),
    }
    let idx_len = bin.len() - pos_len;
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let mut d = doc(
        json!([{"attributes": {"POSITION": 0}, "indices": 1}]),
        nodes,
        bin.len(),
    );
    d["bufferViews"] = json!([
        {"buffer": 0, "byteOffset": 0, "byteLength": pos_len},
        {"buffer": 0, "byteOffset": pos_len, "byteLength": idx_len},
    ]);
    d["accessors"] = json!([
        {"bufferView": 0, "componentType": 5126, "count": 8, "type": "VEC3"},
        {"bufferView": 1, "componentType": index_type, "count": 36, "type": "SCALAR"},
    ]);
    (d, bin)
}

fn glb(json: &Value, bin: &[u8]) -> Vec<u8> {
    let mut j = serde_json::to_vec(json).unwrap();
    while j.len() % 4 != 0 {
        j.push(b' ');
    }
    let mut b = bin.to_vec();
    while b.len() % 4 != 0 {
        b.push(0);
    }
    let total = 12 + 8 + j.len() + if b.is_empty() { 0 } else { 8 + b.len() };
    let mut out = Vec::new();
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((j.len() as u32).to_le_bytes());
    out.extend(0x4E4F_534Au32.to_le_bytes());
    out.extend(j);
    if !b.is_empty() {
        out.extend((b.len() as u32).to_le_bytes());
        out.extend(0x004E_4942u32.to_le_bytes());
        out.extend(b);
    }
    out
}

fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for k in 0..4 {
            if k <= c.len() {
                s.push(A[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn read(name: &str, bytes: &[u8]) -> Result<Mesh, String> {
    read_mesh(name, bytes, Unit::Mm, ImportLimits::default()).map_err(|e| e.to_string())
}

fn signed_volume(m: &Mesh) -> f64 {
    m.triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| m.positions[i as usize].map(f64::from));
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

/// Triangles as corner coordinates in order: the same surface and winding, whatever the vertex numbering.
fn geometry(m: &Mesh) -> Vec<[[f32; 3]; 3]> {
    m.triangles
        .iter()
        .map(|t| t.map(|i| m.positions[i as usize]))
        .collect()
}

fn volume_is(m: &Mesh, want: f64) -> bool {
    (signed_volume(m) - want).abs() <= 1e-9 * want
}

fn bounds(m: &Mesh) -> ([f64; 3], [f64; 3]) {
    let b = m.bounds().unwrap();
    (b.min.map(f64::from), b.max.map(f64::from))
}

fn close3(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() <= tol)
}

#[test]
fn glb_cube_is_read_in_metres_with_outward_winding() {
    let (d, bin) = cube_doc(5125, json!([{"mesh": 0}]));
    let m = read("scan.glb", &glb(&d, &bin)).unwrap();
    assert_eq!((m.positions.len(), m.triangles.len()), (8, 12));
    assert_eq!(bounds(&m), ([0.0; 3], [MM; 3]));
    assert!(volume_is(&m, MM * MM * MM), "{}", signed_volume(&m));
    // The unit option does not apply: glTF is metres by its standard.
    let cm = read_mesh(
        "scan.GLB",
        &glb(&d, &bin),
        Unit::Cm,
        ImportLimits::default(),
    )
    .unwrap();
    assert_eq!(cm, m);
}

#[test]
fn every_index_type_and_no_indices_give_the_same_mesh() {
    let want = read("a.glb", &{
        let (d, b) = cube_doc(5125, json!([{"mesh": 0}]));
        glb(&d, &b)
    })
    .unwrap();
    for ty in [5121, 5123] {
        let (d, b) = cube_doc(ty, json!([{"mesh": 0}]));
        assert_eq!(read("a.glb", &glb(&d, &b)).unwrap(), want, "type {ty}");
    }
    // No indices: 36 corners in triangle order, merged back to 8 vertices.
    let (p, t) = cube();
    let corners: Vec<[f32; 3]> = t.iter().flatten().map(|&i| p[i as usize]).collect();
    let bin = f32s(&corners);
    let mut d = doc(
        json!([{"attributes": {"POSITION": 0}}]),
        json!([{"mesh": 0}]),
        bin.len(),
    );
    d["bufferViews"] = json!([{"buffer": 0, "byteLength": bin.len()}]);
    d["accessors"] = json!([{"bufferView": 0, "componentType": 5126, "count": 36, "type": "VEC3"}]);
    assert_eq!(
        geometry(&read("a.glb", &glb(&d, &bin)).unwrap()),
        geometry(&want)
    );
}

#[test]
fn gltf_with_embedded_buffer_matches_glb_and_stl() {
    let (mut d, bin) = cube_doc(5125, json!([{"mesh": 0}]));
    let from_glb = read("a.glb", &glb(&d, &bin)).unwrap();
    d["buffers"][0]["uri"] = json!(format!(
        "data:application/octet-stream;base64,{}",
        base64(&bin)
    ));
    let from_gltf = read("a.gltf", &serde_json::to_vec(&d).unwrap()).unwrap();
    assert_eq!(from_gltf, from_glb);
    // The same triangles as a binary STL in metres.
    let (p, t) = cube();
    let mut stl = vec![0u8; 80];
    stl.extend(12u32.to_le_bytes());
    for tri in &t {
        stl.extend([0f32; 3].iter().flat_map(|x| x.to_le_bytes()));
        stl.extend(f32s(&tri.map(|i| p[i as usize])));
        stl.extend([0u8; 2]);
    }
    let from_stl = read_mesh("a.stl", &stl, Unit::M, ImportLimits::default()).unwrap();
    assert_eq!(geometry(&from_stl), geometry(&from_glb));
}

#[test]
fn node_transforms_apply_down_the_tree() {
    // Parent: translate (0.5, 0, 0) m. Child: scale 2, then rotate 90 degrees about +z.
    // A cube corner (x, y, z) goes to (0.5 - 2y, 2x, 2z) m.
    let h = std::f32::consts::FRAC_1_SQRT_2;
    let trs = json!([
        {"translation": [0.5, 0.0, 0.0], "children": [1]},
        {"mesh": 0, "rotation": [0.0, 0.0, h, h], "scale": [2.0, 2.0, 2.0]},
    ]);
    let (d, bin) = cube_doc(5125, trs);
    let m = read("a.glb", &glb(&d, &bin)).unwrap();
    let (lo, hi) = bounds(&m);
    assert!(close3(lo, [500.0 - 2.0 * MM, 0.0, 0.0], 1e-4), "{lo:?}");
    assert!(close3(hi, [500.0, 2.0 * MM, 2.0 * MM], 1e-4), "{hi:?}");
    assert!((signed_volume(&m) - 8.0 * MM * MM * MM).abs() < 1.0);
    // The same transform as one column-major matrix on a single node.
    let mat = json!([{"mesh": 0, "matrix": [0, 2, 0, 0, -2, 0, 0, 0, 0, 0, 2, 0, 0.5, 0, 0, 1]}]);
    let (d2, bin2) = cube_doc(5125, mat);
    let m2 = read("a.glb", &glb(&d2, &bin2)).unwrap();
    let (lo2, hi2) = bounds(&m2);
    assert!(
        close3(lo, lo2, 1e-4) && close3(hi, hi2, 1e-4),
        "{lo2:?} {hi2:?}"
    );
}

#[test]
fn a_mirroring_transform_keeps_normals_outward() {
    let (d, bin) = cube_doc(5125, json!([{"mesh": 0, "scale": [-1.0, 1.0, 1.0]}]));
    let m = read("a.glb", &glb(&d, &bin)).unwrap();
    assert_eq!(bounds(&m), ([-MM, 0.0, 0.0], [0.0, MM, MM]));
    assert!(
        volume_is(&m, MM * MM * MM),
        "winding must be reversed: {}",
        signed_volume(&m)
    );
}

#[test]
fn strips_and_fans_make_consistently_wound_triangles() {
    // A unit square in z = 0 as corners 0..4 around it: (0,0) (1,0) (1,1) (0,1).
    let sq = [[0.0, 0.0, 0.0], [S, 0.0, 0.0], [S, S, 0.0], [0.0, S, 0.0]];
    // Strip order (0,0) (1,0) (0,1) (1,1); fan order is the loop itself.
    for (mode, order) in [(5u32, [0usize, 1, 3, 2]), (6, [0, 1, 2, 3])] {
        let pts: Vec<[f32; 3]> = order.iter().map(|&i| sq[i]).collect();
        let bin = f32s(&pts);
        let mut d = doc(
            json!([{"attributes": {"POSITION": 0}, "mode": mode}]),
            json!([{"mesh": 0}]),
            bin.len(),
        );
        d["bufferViews"] = json!([{"buffer": 0, "byteLength": bin.len()}]);
        d["accessors"] =
            json!([{"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3"}]);
        let m = read("a.glb", &glb(&d, &bin)).unwrap();
        assert_eq!(m.triangles.len(), 2, "mode {mode}");
        for t in &m.triangles {
            let [a, b, c] = t.map(|i| m.positions[i as usize].map(f64::from));
            let nz = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(nz > 0.0, "mode {mode}: triangle {t:?} faces down");
        }
    }
}

#[test]
fn interleaved_positions_and_points_primitives() {
    // Positions interleaved with normals (stride 24), plus a points primitive that adds nothing.
    let (p, t) = cube();
    let mut bin = Vec::new();
    for q in &p {
        bin.extend(f32s(&[*q]));
        bin.extend(f32s(&[[0.0, 0.0, 1.0]]));
    }
    let pos_len = bin.len();
    bin.extend(t.iter().flatten().flat_map(|&i| i.to_le_bytes()));
    let prims = json!([
        {"attributes": {"POSITION": 0}, "indices": 1},
        {"attributes": {"POSITION": 0}, "mode": 0},
    ]);
    let mut d = doc(prims, json!([{"mesh": 0}]), bin.len());
    d["bufferViews"] = json!([
        {"buffer": 0, "byteLength": pos_len, "byteStride": 24},
        {"buffer": 0, "byteOffset": pos_len, "byteLength": 144},
    ]);
    d["accessors"] = json!([
        {"bufferView": 0, "componentType": 5126, "count": 8, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5125, "count": 36, "type": "SCALAR"},
    ]);
    let m = read("a.glb", &glb(&d, &bin)).unwrap();
    let (d0, b0) = cube_doc(5125, json!([{"mesh": 0}]));
    assert_eq!(m, read("a.glb", &glb(&d0, &b0)).unwrap());
}

#[test]
fn a_file_without_scenes_reads_each_mesh_once() {
    let (mut d, bin) = cube_doc(5125, json!([{"mesh": 0, "translation": [9.0, 9.0, 9.0]}]));
    d.as_object_mut().unwrap().remove("scene");
    d.as_object_mut().unwrap().remove("scenes");
    let m = read("a.glb", &glb(&d, &bin)).unwrap();
    assert_eq!(bounds(&m), ([0.0; 3], [MM; 3]));
}

#[test]
fn bad_files_are_refused_plainly() {
    let (d, bin) = cube_doc(5125, json!([{"mesh": 0}]));
    let good = glb(&d, &bin);
    let mut cases: Vec<(&str, Vec<u8>, &str)> = vec![];
    let mut v = good.clone();
    v[0] = b'x';
    cases.push(("a.glb", v, "GLB"));
    let mut v = good.clone();
    v[4] = 1;
    cases.push(("a.glb", v, "version"));
    cases.push(("a.glb", good[..good.len() - 7].to_vec(), "length"));
    let edit = |f: &dyn Fn(&mut Value)| {
        let mut d2 = d.clone();
        f(&mut d2);
        glb(&d2, &bin)
    };
    cases.push((
        "a.glb",
        edit(&|d| d["extensionsRequired"] = json!(["KHR_draco_mesh_compression"])),
        "KHR_draco_mesh_compression",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["asset"]["version"] = json!("1.0")),
        "version",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["accessors"][1]["count"] = json!(39)),
        "range",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["accessors"][0]["byteOffset"] = json!(8)),
        "range",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["accessors"][0]["componentType"] = json!(5123)),
        "float",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["accessors"][0]["sparse"] = json!({"count": 1})),
        "sparse",
    ));
    cases.push((
        "a.glb",
        edit(&|d| {
            d["nodes"] = json!([{"children": [1, 1]}, {"mesh": 0}]);
        }),
        "more than once",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["nodes"] = json!([{"mesh": 0, "children": [0]}])),
        "more than once",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["meshes"][0]["primitives"][0]["mode"] = json!(1)),
        "no triangles",
    ));
    cases.push((
        "a.glb",
        edit(&|d| d["nodes"][0]["scale"] = json!([1.0, 1e39, 1.0])),
        "finite",
    ));
    // Index pointing past the 8 positions.
    let mut bad = bin.clone();
    bad[96..100].copy_from_slice(&8u32.to_le_bytes());
    cases.push(("a.glb", glb(&d, &bad), "index"));
    let mut ext = d.clone();
    ext["buffers"][0]["uri"] = json!("scan.bin");
    cases.push(("a.gltf", serde_json::to_vec(&ext).unwrap(), ".glb"));
    cases.push(("a.gltf", b"{not json".to_vec(), "JSON"));
    for (name, bytes, want) in cases {
        let e = read(name, &bytes).unwrap_err();
        assert!(e.contains(want), "want `{want}` in: {e}");
    }
}

#[test]
fn limits_count_instanced_triangles_before_building() {
    // One mesh used by three nodes: 36 triangles expanded.
    let nodes = json!([{"children": [1, 2, 3]}, {"mesh": 0}, {"mesh": 0, "translation": [1, 0, 0]}, {"mesh": 0, "translation": [2, 0, 0]}]);
    let (d, bin) = cube_doc(5125, nodes);
    let bytes = glb(&d, &bin);
    let lim = |t| ImportLimits {
        max_triangles: t,
        ..ImportLimits::default()
    };
    assert_eq!(
        read_mesh("a.glb", &bytes, Unit::Mm, lim(36))
            .unwrap()
            .triangles
            .len(),
        36
    );
    let e = read_mesh("a.glb", &bytes, Unit::Mm, lim(35))
        .unwrap_err()
        .to_string();
    assert!(e.contains("limit"), "{e}");
}

/// Review finding: a small file whose many primitives share one accessor must hit the limit from the
/// accessor counts, before any primitive is decoded.
#[test]
fn shared_accessors_are_limited_before_decoding() {
    let (mut d, bin) = cube_doc(5125, json!([{"mesh": 0}]));
    let prim = json!({"attributes": {"POSITION": 0}, "indices": 1});
    d["meshes"][0]["primitives"] = Value::Array(vec![prim; 1000]);
    let bytes = glb(&d, &bin);
    let lim = |t| ImportLimits {
        max_triangles: t,
        ..ImportLimits::default()
    };
    // 1000 primitives x 12 triangles.
    assert_eq!(
        read_mesh("a.glb", &bytes, Unit::Mm, lim(12_000))
            .unwrap()
            .triangles
            .len(),
        12_000
    );
    let e = read_mesh("a.glb", &bytes, Unit::Mm, lim(11_999))
        .unwrap_err()
        .to_string();
    assert!(e.contains("12000 triangles"), "{e}");
    let v = read_mesh(
        "a.glb",
        &bytes,
        Unit::Mm,
        ImportLimits {
            max_vertices: 7_999,
            ..ImportLimits::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(v.contains("8000 vertices"), "{v}");
}

/// Review finding: the public reader enforces the byte limit itself, like the other readers.
#[test]
fn the_reader_checks_the_byte_limit() {
    let (d, bin) = cube_doc(5125, json!([{"mesh": 0}]));
    let bytes = glb(&d, &bin);
    let lim = ImportLimits {
        max_bytes: bytes.len() - 1,
        ..ImportLimits::default()
    };
    let e = rulercraft_mesh::io::gltf::read_gltf(&bytes, true, lim)
        .unwrap_err()
        .to_string();
    assert!(e.contains("byte limit"), "{e}");
}

/// Review finding: a buffer is only as long as it declares; bytes past that (padding, or a longer BIN
/// chunk) are not geometry.
#[test]
fn buffer_views_stop_at_the_declared_length() {
    let (mut d, bin) = cube_doc(5125, json!([{"mesh": 0}]));
    d["buffers"][0]["byteLength"] = json!(bin.len() - 4);
    let e = read("a.glb", &glb(&d, &bin)).unwrap_err();
    assert!(e.contains("out of range"), "{e}");
}
