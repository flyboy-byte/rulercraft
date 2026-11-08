// SPDX-License-Identifier: MIT OR Apache-2.0
//! io.stl math checks: shapes whose counts and bounds are known exactly.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
use rulercraft_mesh::{Limits, ReadError, Unit, read_stl};

/// The 12 triangles of the unit cube [0,1]^3 (each corner listed once per triangle).
fn cube() -> Vec<[[f32; 3]; 3]> {
    let c = |i: u32| [(i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32];
    let quads = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    quads
        .iter()
        .flat_map(|q| [[c(q[0]), c(q[1]), c(q[2])], [c(q[0]), c(q[2]), c(q[3])]])
        .collect()
}

fn ascii(tris: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let mut s = String::from("solid cube\n");
    for t in tris {
        s += "  facet normal 0 0 0\n    outer loop\n";
        for p in t {
            s += &format!("      vertex {} {} {}\n", p[0], p[1], p[2]);
        }
        s += "    endloop\n  endfacet\n";
    }
    s += "endsolid cube\n";
    s.into_bytes()
}

fn binary(tris: &[[[f32; 3]; 3]], header: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 80];
    b[..header.len()].copy_from_slice(header);
    b.extend((tris.len() as u32).to_le_bytes());
    for t in tris {
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

fn read(bytes: &[u8]) -> Result<rulercraft_mesh::Mesh, ReadError> {
    read_stl(bytes, Unit::Mm, Limits::default())
}

#[test]
fn cube_reads_as_8_vertices_12_triangles_both_encodings() {
    for bytes in [
        ascii(&cube()),
        binary(&cube(), b""),
        binary(&cube(), b"solid but binary"),
    ] {
        let m = read(&bytes).unwrap();
        assert_eq!(m.positions.len(), 8);
        assert_eq!(m.triangles.len(), 12);
        let b = m.bounds().unwrap();
        assert_eq!(b.min, [0.0; 3]);
        assert_eq!(b.max, [1.0; 3]);
    }
}

#[test]
fn units_scale_to_millimetres() {
    let m = read_stl(&ascii(&cube()), Unit::In, Limits::default()).unwrap();
    assert_eq!(m.bounds().unwrap().max, [25.4; 3]);
    let m = read_stl(&binary(&cube(), b""), Unit::Cm, Limits::default()).unwrap();
    assert_eq!(m.bounds().unwrap().max, [10.0; 3]);
}

#[test]
fn negative_zero_merges_with_zero_and_degenerates_are_dropped() {
    let tris = [
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        [[-0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        [[1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
    ];
    let m = read(&binary(&tris, b"")).unwrap();
    assert_eq!(m.positions.len(), 3);
    assert_eq!(m.triangles.len(), 2);
    assert_eq!(m.degenerate_dropped, 1);
}

#[test]
fn polygon_loops_become_fans() {
    let text = "solid q\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 1 1 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid\n";
    let m = read(text.as_bytes()).unwrap();
    assert_eq!((m.positions.len(), m.triangles.len()), (4, 2));
}

#[test]
fn refusals_say_what_is_wrong() {
    assert_eq!(read(b""), Err(ReadError::Empty));
    assert_eq!(read(b"solid empty\nendsolid\n"), Err(ReadError::Empty));
    let mut cut = binary(&cube(), b"");
    cut.truncate(cut.len() - 7);
    assert!(matches!(
        read(&cut),
        Err(ReadError::BinaryLength { claimed: 12, .. })
    ));
    let nan = binary(
        &[[[f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]],
        b"",
    );
    assert_eq!(read(&nan), Err(ReadError::NotFinite { triangle: 1 }));
    let bad = b"solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0\n";
    assert!(matches!(read(bad), Err(ReadError::Syntax { line: 4, .. })));
    let small = Limits {
        max_bytes: 100,
        max_triangles: 10,
    };
    assert!(matches!(
        read_stl(&binary(&cube(), b""), Unit::Mm, small),
        Err(ReadError::TooLarge { .. })
    ));
    let few = Limits {
        max_bytes: 1 << 20,
        max_triangles: 10,
    };
    assert!(matches!(
        read_stl(&ascii(&cube()), Unit::Mm, few),
        Err(ReadError::TooManyTriangles { .. })
    ));
    assert!(
        read(b"solid x\nendloop\n")
            .unwrap_err()
            .to_string()
            .starts_with("line 2:")
    );
}

#[test]
fn a_coordinate_that_overflows_after_unit_conversion_is_refused() {
    let big = binary(
        &[[[3.0e38, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]],
        b"",
    );
    assert!(read(&big).is_ok(), "fine in millimetres");
    assert_eq!(
        read_stl(&big, Unit::In, Limits::default()),
        Err(ReadError::NotFinite { triangle: 1 })
    );
}

#[test]
fn dispatcher_enforces_unique_vertex_limit_before_later_bad_data() {
    use rulercraft_mesh::{ImportLimits, read_mesh};
    let original = cube();
    let mut bad_later = original.clone();
    bad_later.last_mut().unwrap()[2][0] = f32::NAN;
    let limited = ImportLimits {
        max_vertices: 3,
        ..Default::default()
    };
    for (good, bad) in [
        (ascii(&original), ascii(&bad_later)),
        (binary(&original, b""), binary(&bad_later, b"")),
    ] {
        let error = read_mesh("cube.stl", &bad, Unit::Mm, limited)
            .unwrap_err()
            .to_string();
        assert!(error.contains("vertex limit"), "{error}");
        let too_small = ImportLimits {
            max_vertices: 7,
            ..Default::default()
        };
        assert!(
            read_mesh("cube.stl", &good, Unit::Mm, too_small)
                .unwrap_err()
                .to_string()
                .contains("vertex limit")
        );
        let exact = ImportLimits {
            max_vertices: 8,
            ..Default::default()
        };
        assert_eq!(
            read_mesh("cube.stl", &good, Unit::Mm, exact)
                .unwrap()
                .positions
                .len(),
            8
        );
    }
}

#[test]
fn ascii_triangle_limit_counts_degenerate_fan_candidates_before_later_input() {
    let prefix = "solid t\nouter loop\nvertex 0 0 0\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\n";
    let complete = format!("{prefix}endloop\nendsolid\n");
    let bad_later = format!("{prefix}vertex NaN 0 0\nendloop\nendsolid\n");
    let limits = |max_triangles| Limits {
        max_triangles,
        ..Default::default()
    };
    assert_eq!(
        read_stl(bad_later.as_bytes(), Unit::Mm, limits(1)),
        Err(ReadError::TooManyTriangles {
            triangles: 2,
            limit: 1
        })
    );
    assert_eq!(
        read_stl(complete.as_bytes(), Unit::Mm, limits(0)),
        Err(ReadError::TooManyTriangles {
            triangles: 1,
            limit: 0
        })
    );
    let mesh = read_stl(complete.as_bytes(), Unit::Mm, limits(2)).unwrap();
    assert_eq!(
        (
            mesh.positions.len(),
            mesh.triangles.len(),
            mesh.degenerate_dropped
        ),
        (3, 1, 1)
    );
}
