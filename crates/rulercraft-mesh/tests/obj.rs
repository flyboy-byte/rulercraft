// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact synthetic geometry checks for OBJ import.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
fn read(s: &str) -> Result<rulercraft_mesh::Mesh, rulercraft_mesh::io::ReadError> {
    read_mesh("cube.OBJ", s.as_bytes(), Unit::Mm, ImportLimits::default())
}
fn cube() -> String {
    let mut s = String::from("# unit cube\no cube\n");
    for i in 0..8 {
        s.push_str(&format!("v {} {} {}\n", i & 1, (i >> 1) & 1, (i >> 2) & 1));
    }
    s.push_str("f 1 3 4 2\nf 5 6 8 7\nf 1 2 6 5\nf 3 7 8 4\nf 1 5 7 3\nf -7 -5 -1 -3\n");
    s
}
#[test]
fn cube_exact_counts_bounds_and_units() {
    let s = cube();
    let m = read(&s).unwrap();
    assert_eq!((m.positions.len(), m.triangles.len()), (8, 12));
    assert_eq!(m.bounds().unwrap().min, [0.; 3]);
    assert_eq!(m.bounds().unwrap().max, [1.; 3]);
    assert_eq!(&m.triangles[..2], &[[0, 2, 3], [0, 3, 1]]);
    assert_eq!(
        read_mesh("a.obj", s.as_bytes(), Unit::Cm, ImportLimits::default())
            .unwrap()
            .bounds()
            .unwrap()
            .max,
        [10.; 3]
    );
}
#[test]
fn ngon_slashes_continuation_and_merging() {
    let m=read("v 0 0 0\nv 1 0 0 1\nv 1 1 0\nv 0 1 0\nv -0 0 0\nvt 0 0\nvn 0 0 1\nf 1/1 2//1 3/1/1 \\\n4 5 # fan\nf 1 5 2\n").unwrap();
    assert_eq!(
        (m.positions.len(), m.triangles.len(), m.degenerate_dropped),
        (4, 2, 2)
    );
    assert_eq!(m.triangles, [[0, 1, 2], [0, 2, 3]]);
}
#[test]
fn clean_refusals_and_limits() {
    for s in [
        "v NaN 0 0",
        "v 0 0",
        "v 0 0 0 2",
        "v 0 0 0\nf 0 1 1",
        "v 0 0 0\nf -2 1 1",
        "v 0 0 0\nf 2 1 1",
        "v 0 0 0\nf 1 1",
        "curv 1",
        "v 0 0 0\nf 1/ 1 1",
        "v 0 0 0\nf 1/0 1 1",
        "v 0 0 0\\",
    ] {
        assert!(read(s).unwrap_err().to_string().contains("line"), "{s}");
    }
    for limits in [
        ImportLimits {
            max_bytes: 1,
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
            read_mesh("a.obj", cube().as_bytes(), Unit::Mm, limits)
                .unwrap_err()
                .to_string()
                .contains("limit")
        );
    }
    assert!(read("").is_err());
    assert!(read("v 0 0 0\n").is_err());
    assert!(
        read_mesh("a.obj", b"\xff", Unit::Mm, ImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("offset")
    );
    assert!(
        read_mesh("a.obj", b"v 3e38 0 0", Unit::M, ImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("line")
    );
}
