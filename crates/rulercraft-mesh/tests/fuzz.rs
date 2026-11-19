// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fuzzing every reader and the command layer, with no fuzzing library: a seeded mutator over small valid files of
//! every format (bit flips, interesting bytes and integers, inserted, deleted and repeated runs, spliced numbers,
//! truncation), and random fit requests over a known mesh. Every outcome must be a clean refusal or a mesh whose
//! numbers are finite, whose indices are in range and whose sizes are within the limits given; nothing may panic or
//! run long.
//!
//! The fast run below is part of every test run. For a long run (T2, docs/specs/verification.md), by hand:
//! `RULERCRAFT_FUZZ_SECONDS=3600 cargo test --release -p rulercraft-mesh --test fuzz -- --ignored`
//! (`RULERCRAFT_FUZZ_SEED` picks another sequence; a failure prints the seed and iteration that made it).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
use rulercraft_mesh::commands::{FitKind, FitRequest, Selection, fit};
use rulercraft_mesh::fit::Clip;
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::{ImportLimits, Mesh, Unit, read_cloud, read_mesh};
use std::io::{Cursor, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

/// xorshift64*: small, seeded, the same everywhere.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn f(&mut self) -> f64 {
        const ODD: [f64; 9] = [
            0.0,
            -0.0,
            1e-300,
            1e300,
            f64::NAN,
            f64::INFINITY,
            -1.0,
            1e9,
            0.5,
        ];
        match self.below(4) {
            0 => ODD[self.below(ODD.len())],
            _ => (self.next() % 200_000) as f64 / 1000.0 - 100.0,
        }
    }
}

fn binary_stl() -> Vec<u8> {
    let mut b = vec![0u8; 80];
    b.extend(2u32.to_le_bytes());
    for t in [
        [[0f32, 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        [[1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
    ] {
        b.extend([0u8; 12]);
        for p in t {
            for v in p {
                b.extend(v.to_le_bytes());
            }
        }
        b.extend([0u8; 2]);
    }
    b
}

fn ply(enc: &str) -> Vec<u8> {
    let mut b = format!(
        "ply\nformat {enc} 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\n\
         element face 2\nproperty list uchar int vertex_indices\nend_header\n"
    )
    .into_bytes();
    let pts = [0f32, 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0.];
    if enc == "ascii" {
        b.extend(b"0 0 0\n1 0 0\n1 1 0\n0 1 0\n3 0 1 2\n4 0 2 3 1\n");
    } else {
        for v in pts {
            b.extend(v.to_le_bytes());
        }
        for f in [&[0i32, 1, 2][..], &[0, 2, 3, 1]] {
            b.push(f.len() as u8);
            for i in f {
                b.extend(i.to_le_bytes());
            }
        }
    }
    b
}

fn threemf() -> Vec<u8> {
    let model = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
<resources><object id="1" type="model"><mesh><vertices>
<vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/><vertex x="0" y="1" z="0"/><vertex x="0" y="0" z="1"/>
</vertices><triangles><triangle v1="0" v2="2" v3="1"/><triangle v1="0" v2="1" v3="3"/>
<triangle v1="0" v2="3" v3="2"/><triangle v1="1" v2="2" v3="3"/></triangles></mesh></object></resources>
<build><item objectid="1"/></build></model>"#;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("3D/3dmodel.model", options).unwrap();
    zip.write_all(model.as_bytes()).unwrap();
    zip.finish().unwrap().into_inner()
}

/// A tetrahedron as glTF: positions and indices in one buffer, as embedded base64 (`gltf`) or a binary chunk (`glb`).
fn gltf(binary: bool) -> Vec<u8> {
    let mut bin = Vec::new();
    for v in [0f32, 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.] {
        bin.extend(v.to_le_bytes());
    }
    for i in [0u16, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3] {
        bin.extend(i.to_le_bytes());
    }
    let uri = if binary {
        String::new()
    } else {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::from(r#","uri":"data:application/octet-stream;base64,"#);
        for c in bin.chunks(3) {
            let n = (u32::from(c[0]) << 16)
                | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
                | u32::from(*c.get(2).unwrap_or(&0));
            for k in 0..4 {
                s.push(if k <= c.len() {
                    A[(n >> (18 - 6 * k) & 63) as usize] as char
                } else {
                    '='
                });
            }
        }
        s.push('"');
        s
    };
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],
"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1}}]}}],
"buffers":[{{"byteLength":{len}{uri}}}],
"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":48}},{{"buffer":0,"byteOffset":48,"byteLength":24}}],
"accessors":[{{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3","min":[0,0,0],"max":[1,1,1]}},
{{"bufferView":1,"componentType":5123,"count":12,"type":"SCALAR"}}]}}"#,
        len = bin.len()
    );
    if !binary {
        return json.into_bytes();
    }
    let mut j = json.into_bytes();
    while j.len() % 4 != 0 {
        j.push(b' ');
    }
    let total = 12 + 8 + j.len() + 8 + bin.len();
    let mut out = Vec::new();
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((j.len() as u32).to_le_bytes());
    out.extend(0x4E4F_534Au32.to_le_bytes());
    out.extend(j);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004E_4942u32.to_le_bytes());
    out.extend(bin);
    out
}

/// Every seed, by the file name its reader is chosen from, and whether it is read as a cloud.
fn seeds() -> Vec<(&'static str, bool, Vec<u8>)> {
    let ascii_stl = b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
    vec![
        ("a.stl", false, ascii_stl.to_vec()),
        ("b.stl", false, binary_stl()),
        ("a.ply", false, ply("ascii")),
        ("b.ply", false, ply("binary_little_endian")),
        ("p.ply", true, ply("binary_little_endian")),
        (
            "a.obj",
            false,
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nvn 0 0 1\nf 1//1 2//1 3//1\nf 2 4 3\n".to_vec(),
        ),
        ("a.3mf", false, threemf()),
        ("a.gltf", false, gltf(false)),
        ("a.glb", false, gltf(true)),
        ("a.xyz", true, b"X,Y,Z\n0,0,0\n1 0 0\n0;1;0 9 9\n".to_vec()),
        (
            "a.pts",
            true,
            b"3\n0 0 0 1 2 3\n1 0 0 1 2 3\n0 1 0 1 2 3\n".to_vec(),
        ),
    ]
}

fn mutate(rng: &mut Rng, mut b: Vec<u8>) -> Vec<u8> {
    const BYTES: [u8; 12] = [
        0, 0xff, 0x7f, 0x80, b'-', b'9', b'\n', b' ', b'"', b'<', b'{', b'.',
    ];
    const INTS: [u32; 8] = [0, 1, 3, 0xff, 0xffff, 0x7fff_ffff, 0xffff_ffff, 1 << 30];
    const WORDS: [&[u8]; 10] = [
        b"1e309",
        b"-0",
        b"nan",
        b"inf",
        b"4294967295",
        b"-1",
        b"99999999999999999999",
        b"0x10",
        b"\x00",
        b"end_header",
    ];
    for _ in 0..1 + rng.below(4) {
        let n = b.len();
        match rng.below(8) {
            0 if n > 0 => {
                let i = rng.below(n);
                b[i] ^= 1 << rng.below(8);
            }
            1 if n > 0 => {
                let i = rng.below(n);
                b[i] = BYTES[rng.below(BYTES.len())];
            }
            2 if n >= 4 => {
                let i = rng.below(n - 3);
                b[i..i + 4].copy_from_slice(&INTS[rng.below(INTS.len())].to_le_bytes());
            }
            3 => {
                let i = rng.below(n + 1);
                let w = WORDS[rng.below(WORDS.len())];
                b.splice(i..i, w.iter().copied());
            }
            4 if n > 0 => {
                let i = rng.below(n);
                let len = 1 + rng.below((n - i).min(64));
                b.drain(i..i + len);
            }
            5 if n > 0 => {
                // Repeat a run, up to 64 times: many records, deep nesting, long lines.
                let i = rng.below(n);
                let len = 1 + rng.below((n - i).min(32));
                let run: Vec<u8> = b[i..i + len].to_vec();
                for _ in 0..1 + rng.below(64) {
                    b.splice(i..i, run.iter().copied());
                }
            }
            6 => b.truncate(rng.below(n + 1)),
            _ => {
                let i = rng.below(n + 1);
                let extra: Vec<u8> = (0..1 + rng.below(16)).map(|_| rng.next() as u8).collect();
                b.splice(i..i, extra);
            }
        }
    }
    b
}

fn limits() -> ImportLimits {
    ImportLimits {
        max_bytes: 1 << 20,
        max_vertices: 100_000,
        max_triangles: 100_000,
    }
}

fn check(name: &str, mesh: &Mesh) {
    let l = limits();
    assert!(
        mesh.positions.len() <= l.max_vertices,
        "{name}: {} vertices",
        mesh.positions.len()
    );
    assert!(
        mesh.triangles.len() <= l.max_triangles,
        "{name}: {} triangles",
        mesh.triangles.len()
    );
    assert!(
        mesh.positions.iter().flatten().all(|v| v.is_finite()),
        "{name}: a position is not finite"
    );
    let n = mesh.positions.len() as u32;
    assert!(
        mesh.triangles.iter().flatten().all(|&i| i < n),
        "{name}: a triangle index is out of range"
    );
}

/// Fuzzes every reader until `iterations` inputs per seed are done or `budget` runs out.
fn fuzz_readers(seed: u64, iterations: usize, budget: Duration) -> usize {
    let mut rng = Rng(seed | 1);
    let start = Instant::now();
    let mut done = 0;
    let seeds = seeds();
    for it in 0..iterations {
        for (name, cloud, bytes) in &seeds {
            if start.elapsed() > budget {
                return done;
            }
            let input = mutate(&mut rng, bytes.clone());
            let t = Instant::now();
            let result = catch_unwind(AssertUnwindSafe(|| {
                if *cloud {
                    read_cloud(name, &input, Unit::Mm, limits())
                } else {
                    read_mesh(name, &input, Unit::Mm, limits())
                }
            }));
            let took = t.elapsed();
            match result {
                Ok(Ok(mesh)) => check(name, &mesh),
                Ok(Err(e)) => assert!(!e.to_string().is_empty(), "{name}: an empty refusal"),
                Err(_) => panic!("{name} panicked: seed {seed}, iteration {it}, input {input:?}"),
            }
            assert!(
                took < Duration::from_secs(5),
                "{name}: {took:?} on seed {seed}, iteration {it}"
            );
            done += 1;
        }
    }
    done
}

/// A small closed mesh to fit on: an octahedron.
fn octahedron() -> Mesh {
    let positions = vec![
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    let triangles = vec![
        [0, 2, 4],
        [2, 1, 4],
        [1, 3, 4],
        [3, 0, 4],
        [2, 0, 5],
        [1, 2, 5],
        [3, 1, 5],
        [0, 3, 5],
    ];
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

/// Fuzzes the command layer: random selections (including non-finite, inverted and huge), kinds, clips and known
/// diameters. Each must be a refusal or a fit whose numbers are finite.
fn fuzz_commands(seed: u64, iterations: usize, budget: Duration) -> usize {
    let mut rng = Rng(seed | 1);
    let mesh = octahedron();
    let graph = MeshGraph::build(&mesh);
    let start = Instant::now();
    let mut done = 0;
    for it in 0..iterations {
        if start.elapsed() > budget {
            break;
        }
        let v3 = |rng: &mut Rng| [rng.f(), rng.f(), rng.f()];
        let mut selection = Selection::default();
        if rng.below(2) == 0 {
            selection.in_box = Some((v3(&mut rng), v3(&mut rng)));
        }
        if rng.below(2) == 0 {
            selection.near = Some((v3(&mut rng), rng.f()));
        }
        if rng.below(2) == 0 {
            selection.facing = Some((v3(&mut rng), rng.f()));
        }
        let request = FitRequest {
            kind: [FitKind::Plane, FitKind::Sphere, FitKind::Cylinder][rng.below(3)],
            selection,
            clip: if rng.below(2) == 0 {
                Clip::None
            } else {
                Clip::Sigma(rng.below(6) as u8)
            },
            known_diameter: (rng.below(3) == 0).then(|| rng.f()),
        };
        let result = catch_unwind(AssertUnwindSafe(|| fit(&mesh, &graph, &request)));
        match result {
            Ok(Ok(out)) => {
                let text = serde_json::to_string(&out.fitted).unwrap();
                assert!(
                    !text.contains("null")
                        || text.contains("\"free_diameter\":null")
                        || text.contains("_diameter\":null"),
                    "a non-finite number in a fit: {text} for {request:?}"
                );
            }
            Ok(Err(e)) => assert!(!e.to_string().is_empty()),
            Err(_) => panic!("fit panicked: seed {seed}, iteration {it}, {request:?}"),
        }
        done += 1;
    }
    done
}

fn seed() -> u64 {
    std::env::var("RULERCRAFT_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5EED_2026_1119)
}

#[test]
fn readers_and_commands_survive_a_short_fuzz() {
    let r = fuzz_readers(seed(), 400, Duration::from_secs(60));
    let c = fuzz_commands(seed(), 2000, Duration::from_secs(60));
    assert!(r >= 400 && c >= 2000, "{r} reader inputs, {c} commands");
}

#[test]
#[ignore = "long fuzz run, by hand: RULERCRAFT_FUZZ_SECONDS"]
fn readers_and_commands_survive_a_long_fuzz() {
    let secs: u64 = std::env::var("RULERCRAFT_FUZZ_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    let half = Duration::from_secs(secs / 2);
    let r = fuzz_readers(seed(), usize::MAX, half);
    let c = fuzz_commands(seed(), usize::MAX, half);
    eprintln!(
        "fuzzed {r} reader inputs and {c} commands from seed {}",
        seed()
    );
}
