// SPDX-License-Identifier: MIT OR Apache-2.0
//! Readers must refuse corrupted files, never panic: every prefix and thousands of byte-flipped
//! copies of small valid STL, PLY and OBJ files. Runs in well under a second.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};

fn samples() -> Vec<(&'static str, Vec<u8>)> {
    let ascii_stl = b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid\n".to_vec();
    let mut bin_stl = vec![0u8; 80];
    bin_stl.extend(2u32.to_le_bytes());
    for t in [
        [[0f32, 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        [[1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
    ] {
        bin_stl.extend([0u8; 12]);
        for p in t {
            for v in p {
                bin_stl.extend(v.to_le_bytes());
            }
        }
        bin_stl.extend([0u8; 2]);
    }
    let head = |enc: &str| {
        format!("ply\nformat {enc} 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 2\nproperty list uchar int vertex_indices\nend_header\n").into_bytes()
    };
    let mut ply_ascii = head("ascii");
    ply_ascii.extend(b"0 0 0\n1 0 0\n1 1 0\n0 1 0\n3 0 1 2\n4 0 2 3 1\n");
    let mut ply_be = head("binary_big_endian");
    for v in [0f32, 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0.] {
        ply_be.extend(v.to_be_bytes());
    }
    for f in [[0i32, 1, 2], [0, 2, 3]] {
        ply_be.push(3);
        for i in f {
            ply_be.extend(i.to_be_bytes());
        }
    }
    let obj = b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvn 0 0 1\nf 1/1/1 2/1/1 3/1/1 4/1/1\nf -4 -2 -1\n".to_vec();
    vec![
        ("a.stl", ascii_stl),
        ("b.stl", bin_stl),
        ("a.ply", ply_ascii),
        ("b.ply", ply_be),
        ("a.obj", obj),
    ]
}

#[test]
fn originals_read() {
    for (name, bytes) in samples() {
        assert!(
            read_mesh(name, &bytes, Unit::Mm, ImportLimits::default()).is_ok(),
            "{name}"
        );
    }
}

#[test]
fn every_prefix_and_thousands_of_flipped_copies_refuse_or_read_without_panicking() {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let limits = ImportLimits {
        max_bytes: 1 << 16,
        max_vertices: 10_000,
        max_triangles: 10_000,
    };
    let mut tried = 0;
    for (name, bytes) in samples() {
        for cut in 0..bytes.len() {
            let _ = read_mesh(name, &bytes[..cut], Unit::In, limits);
            tried += 1;
        }
        for _ in 0..2000 {
            let mut b = bytes.clone();
            for _ in 0..(1 + next() % 4) {
                let i = (next() % b.len() as u64) as usize;
                b[i] = match next() % 4 {
                    0 => 0xff,
                    1 => b'-',
                    2 => (next() & 0xff) as u8,
                    _ => b'9',
                };
            }
            if next() % 3 == 0 {
                b.extend(std::iter::repeat_n(b'7', (next() % 64) as usize));
            }
            let _ = read_mesh(name, &b, Unit::Mm, limits);
            tried += 1;
        }
    }
    assert!(tried > 10_000);
}
