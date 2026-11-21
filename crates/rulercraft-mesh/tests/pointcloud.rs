// SPDX-License-Identifier: MIT OR Apache-2.0
//! Point clouds: text (XYZ, PTS, ASC) and PLY vertices, read as meshes with no triangles and fit directly.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, CylinderOptions, fit_cylinder, fit_plane, fit_sphere};
use rulercraft_mesh::io::{points::read_text_points, read_cloud};
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};

/// Points on a sphere of radius 10 at (1, 2, 3), spread by the golden angle so the set is fixed.
fn sphere_points(n: usize) -> Vec<[f64; 3]> {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    (0..n)
        .map(|i| {
            let z = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
            let r = (1.0 - z * z).sqrt();
            let a = golden * i as f64;
            [
                1.0 + 10.0 * r * a.cos(),
                2.0 + 10.0 * r * a.sin(),
                3.0 + 10.0 * z,
            ]
        })
        .collect()
}

fn all(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

#[test]
fn an_xyz_file_with_a_header_commas_and_extra_columns_fits_its_sphere() {
    let mut text = String::from("# made by a scanner\nX,Y,Z,Intensity\n\n");
    for p in sphere_points(400) {
        text.push_str(&format!("{:.7},{:.7},{:.7},55\n", p[0], p[1], p[2]));
    }
    let m = read_cloud(
        "part.XYZ",
        text.as_bytes(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    assert_eq!((m.positions.len(), m.triangles.len()), (400, 0));
    let f = fit_sphere(&m.positions, &all(400), Clip::None, None).unwrap();
    assert!((f.diameter - 20.0).abs() < 1e-4, "{f:?}");
    assert!((f.center[0] - 1.0).abs() < 1e-4 && (f.center[2] - 3.0).abs() < 1e-4);
}

#[test]
fn a_pts_file_skips_its_count_and_fits_a_cylinder_in_cm() {
    // Radius 0.5 cm, 4 cm long along z; the first line is the point count, then x y z intensity r g b.
    let mut text = String::from("240\n");
    for i in 0..240 {
        let a = i as f64 * 0.37;
        let z = (i % 40) as f64 * 0.1;
        text.push_str(&format!(
            "{:.7} {:.7} {:.7} 120 10 20 30\n",
            0.5 * a.cos(),
            0.5 * a.sin(),
            z
        ));
    }
    let m = read_text_points(text.as_bytes(), Unit::Cm, ImportLimits::default()).unwrap();
    assert_eq!(m.positions.len(), 240);
    let f = fit_cylinder(&m.positions, None, &all(240), CylinderOptions::default()).unwrap();
    assert!((f.diameter - 10.0).abs() < 1e-3, "in mm: {f:?}");
}

#[test]
fn an_asc_file_with_tabs_and_semicolons_fits_its_plane() {
    let text = "x\ty\tz\n0\t0\t5\n10\t0\t5\n0;10;5\n10;10;5\n5 5 5\n";
    let m = read_cloud("a.asc", text.as_bytes(), Unit::Mm, ImportLimits::default()).unwrap();
    let f = fit_plane(&m.positions, None, &all(5), Clip::None).unwrap();
    assert!(
        (f.normal[2].abs() - 1.0).abs() < 1e-9 && (f.center[2] - 5.0).abs() < 1e-9,
        "{f:?}"
    );
}

#[test]
fn a_ply_with_only_vertices_is_a_cloud_and_its_faces_are_left_out() {
    let mut ply = String::from(
        "ply\nformat ascii 1.0\nelement vertex 200\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nend_header\n",
    );
    for p in sphere_points(200) {
        ply.push_str(&format!("{} {} {} 7\n", p[0], p[1], p[2]));
    }
    let m = read_cloud(
        "scan.ply",
        ply.as_bytes(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    assert_eq!((m.positions.len(), m.triangles.len()), (200, 0));
    let f = fit_sphere(&m.positions, &all(200), Clip::None, None).unwrap();
    assert!((f.diameter - 20.0).abs() < 1e-3, "{f:?}");
    // As a mesh it is refused, saying where it can be read.
    let e = read_mesh(
        "scan.ply",
        ply.as_bytes(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap_err();
    assert!(e.to_string().contains("point cloud"), "{e}");
    // A PLY mesh read as a cloud keeps its vertices, not its faces.
    let tri = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n";
    let m = read_cloud("t.ply", tri.as_bytes(), Unit::Mm, ImportLimits::default()).unwrap();
    assert_eq!((m.positions.len(), m.triangles.len()), (3, 0));
}

#[test]
fn bad_point_files_are_refused_plainly() {
    let limits = ImportLimits::default();
    let e = read_text_points(b"1 2 3\n4 5\n", Unit::Mm, limits).unwrap_err();
    assert!(e.to_string().starts_with("line 2:"), "{e}");
    let e = read_text_points(b"1 2 3\n4 five 6\n", Unit::Mm, limits).unwrap_err();
    assert!(e.to_string().starts_with("line 2:"), "{e}");
    let e = read_text_points(b"1 2 inf\n", Unit::Mm, limits).unwrap_err();
    assert!(e.to_string().contains("not finite"), "{e}");
    let e = read_text_points(b"# nothing\nX Y Z\n", Unit::Mm, limits).unwrap_err();
    assert!(e.to_string().contains("no points"), "{e}");
    let tight = ImportLimits {
        max_vertices: 2,
        ..limits
    };
    let e = read_text_points(b"0 0 0\n1 0 0\n2 0 0\n", Unit::Mm, tight).unwrap_err();
    assert!(e.to_string().contains("limit"), "{e}");
    let e = read_text_points(&[b'1', b' ', 0xff, b'\n'], Unit::Mm, limits).unwrap_err();
    assert!(e.to_string().contains("UTF-8"), "{e}");
    // A text cloud named as a mesh to a mesh reader says what to do.
    let e = read_mesh("a.xyz", b"0 0 0\n", Unit::Mm, limits).unwrap_err();
    assert!(e.to_string().contains("`fit` and `measure`"), "{e}");
}

#[test]
fn malformed_first_point_rows_are_refused() {
    for row in ["10 20 bad", "X 20 30", "10 20", "10.5"] {
        let text = format!("# scanner export\n{row}\n0 0 5\n10 0 5\n0 10 5\n");
        let error = read_cloud(
            "bad.xyz",
            text.as_bytes(),
            Unit::Mm,
            ImportLimits::default(),
        )
        .unwrap_err();
        assert!(error.to_string().starts_with("line 2:"), "{row}: {error}");
    }
}
