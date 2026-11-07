// SPDX-License-Identifier: MIT OR Apache-2.0
//! io.ply known-answer checks. Counts and bounds use exact representable coordinates.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};

fn cube(encoding: &str) -> Vec<u8> {
    let mut b = format!("ply\nformat {encoding} 1.0\ncomment synthetic unit cube\nelement vertex 8\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nelement face 6\nproperty list uchar int vertex_indices\nend_header\n").into_bytes();
    for i in 0..8 {
        let p = [(i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32];
        if encoding == "ascii" {
            b.extend(format!("{} {} {} 255\n", p[0], p[1], p[2]).bytes());
        } else {
            for v in p {
                b.extend(if encoding.ends_with("little_endian") {
                    v.to_le_bytes()
                } else {
                    v.to_be_bytes()
                });
            }
            b.push(255);
        }
    }
    for q in [
        [0i32, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        if encoding == "ascii" {
            b.extend(format!("4 {} {} {} {}\n", q[0], q[1], q[2], q[3]).bytes());
        } else {
            b.push(4);
            for i in q {
                b.extend(if encoding.ends_with("little_endian") {
                    i.to_le_bytes()
                } else {
                    i.to_be_bytes()
                });
            }
        }
    }
    b
}
fn read(b: &[u8]) -> Result<rulercraft_mesh::Mesh, rulercraft_mesh::io::ReadError> {
    read_mesh("cube.PLY", b, Unit::Mm, ImportLimits::default())
}
#[test]
fn cube_all_encodings_and_units() {
    for enc in ["ascii", "binary_little_endian", "binary_big_endian"] {
        let b = cube(enc);
        let m = read(&b).unwrap();
        assert_eq!((m.positions.len(), m.triangles.len()), (8, 12));
        assert_eq!(m.bounds().unwrap().min, [0.; 3]);
        assert_eq!(m.bounds().unwrap().max, [1.; 3]);
        assert_eq!(
            read_mesh("a.ply", &b, Unit::Cm, ImportLimits::default())
                .unwrap()
                .bounds()
                .unwrap()
                .max,
            [10.; 3]
        );
        assert_eq!(&m.triangles[..2], &[[0, 2, 3], [0, 3, 1]]);
    }
}
#[test]
fn reordered_properties_ngon_and_merged_degenerates() {
    let b = b"ply\nformat ascii 1.0\nelement vertex 5\nproperty double z\nproperty short x\nproperty float y\nelement face 2\nproperty list uint8 uint32 vertex_index\nend_header\n0 0 0\n0 1 0\n0 1 1\n0 0 1\n0 0 -0\n5 0 1 2 3 4\n3 0 4 1\n";
    let m = read(b).unwrap();
    assert_eq!(
        (m.positions.len(), m.triangles.len(), m.degenerate_dropped),
        (4, 2, 2)
    );
}
#[test]
fn refusals_and_limits_have_locations() {
    for b in [
        cube("ascii"),
        cube("binary_little_endian"),
        cube("binary_big_endian"),
    ] {
        assert!(read(&b[..b.len() - 4]).unwrap_err().to_string().contains(
            if b.starts_with(b"ply\nformat ascii") {
                "line"
            } else {
                "offset"
            }
        ));
        for limits in [
            ImportLimits {
                max_bytes: 10,
                ..Default::default()
            },
            ImportLimits {
                max_vertices: 7,
                ..Default::default()
            },
            ImportLimits {
                max_triangles: 11,
                ..Default::default()
            },
        ] {
            assert!(
                read_mesh("a.ply", &b, Unit::Mm, limits)
                    .unwrap_err()
                    .to_string()
                    .contains("limit")
            );
        }
    }
    let b = String::from_utf8(cube("ascii")).unwrap();
    for bad in [
        b.replacen("0 0 0 255", "NaN 0 0 255", 1),
        b.replacen("4 0 2 3 1", "4 0 2 3 99", 1),
        b.replacen("4 0 2 3 1", "-1", 1),
        format!("{b}junk"),
    ] {
        assert!(
            read(bad.as_bytes())
                .unwrap_err()
                .to_string()
                .contains("line")
        );
    }
    assert!(read(b"").is_err());
    assert!(
        read_mesh("a.unknown", b"", Unit::Mm, ImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains(".ply")
    );
}

#[test]
fn skipped_lists_empty_mesh_and_overflow_refuse_cleanly() {
    let b = String::from_utf8(cube("ascii")).unwrap();
    let b = b
        .replace(
            "element vertex 8",
            "element metadata 1\nproperty list uchar short values\nelement vertex 8",
        )
        .replace("end_header\n", "end_header\n3 -1 0 1\n");
    assert_eq!(read(b.as_bytes()).unwrap().triangles.len(), 12);
    let basic = String::from_utf8(cube("ascii")).unwrap();
    for bad in [
        basic.replacen("property float x", "property float x\nproperty float x", 1),
        basic.replacen("property float x", "property float other", 1),
        basic.replacen("0 0 0 255", "3e38 0 0 255", 1),
    ] {
        assert!(
            read_mesh("a.ply", bad.as_bytes(), Unit::M, ImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("line")
        );
    }
    let empty = b"ply\nformat ascii 1.0\nelement vertex 0\nproperty float x\nproperty float y\nproperty float z\nelement face 0\nproperty list uchar int vertex_indices\nend_header\n";
    assert!(
        read(empty)
            .unwrap_err()
            .to_string()
            .contains("no triangles")
    );
}
