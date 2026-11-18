// SPDX-License-Identifier: MIT OR Apache-2.0
//! A scan as a SolidCraft design (docs/specs/mesh-to-cad.md, "SolidCraft design"). Known answers from
//! fixtures/known: the hole block is a 60 x 30 x 10 plate with through holes of 3, 5 and 8 mm at x = 10,
//! 30, 50 (y = 15); the stepped cylinder has no holes, only two outward round faces.
#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]
use rulercraft_mesh::io::{ImportLimits, read_mesh};
use rulercraft_mesh::solidcraft::design;
use rulercraft_mesh::{Mesh, Unit};
use serde_json::Value;

fn fixture(name: &str) -> Mesh {
    let path = format!("{}/../../fixtures/known/{name}", env!("CARGO_MANIFEST_DIR"));
    read_mesh(
        name,
        &std::fs::read(path).unwrap(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap()
}

fn v3(v: &Value) -> [f64; 3] {
    [0, 1, 2].map(|k| v[k].as_f64().unwrap())
}

/// A point in the design's coordinates, back in the scan's, through the reported frame.
fn to_scan(report: &Value, p: [f64; 3]) -> [f64; 3] {
    let f = &report["details"]["frame"];
    let (o, u, v, n) = (v3(&f["origin"]), v3(&f["u"]), v3(&f["v"]), v3(&f["normal"]));
    let lo = [
        f["outline_min"][0].as_f64().unwrap(),
        f["outline_min"][1].as_f64().unwrap(),
    ];
    let z0 = f["z0"].as_f64().unwrap();
    let (a, b, c) = (p[0] + lo[0], p[1] + lo[1], p[2] + z0);
    [0, 1, 2].map(|k| o[k] + a * u[k] + b * v[k] + c * n[k])
}

fn det(report: &Value) -> f64 {
    let f = &report["details"]["frame"];
    let (u, v, n) = (v3(&f["u"]), v3(&f["v"]), v3(&f["normal"]));
    u[0] * (v[1] * n[2] - v[2] * n[1]) - u[1] * (v[0] * n[2] - v[2] * n[0])
        + u[2] * (v[0] * n[1] - v[1] * n[0])
}

fn param(design: &Value, name: &str) -> String {
    design["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("no parameter {name}"))["expr"]
        .as_str()
        .unwrap()
        .to_string()
}

fn check_hole_block(mesh: &Mesh, back: impl Fn([f64; 3]) -> [f64; 3]) {
    let r = design(mesh, "hole-block").unwrap();
    let (d, rep) = (&r.design, &r.report);
    assert_eq!(d["format"], "solidcraft/57");
    assert_eq!(
        rep["items"].as_array().unwrap().len(),
        0,
        "nothing lost: {}",
        rep["items"]
    );
    // The plate's sides may be either way round; the thickness may not.
    let (l, w) = (param(d, "length"), param(d, "width"));
    assert!(
        (l == "60 mm" && w == "30 mm") || (l == "30 mm" && w == "60 mm"),
        "{l} x {w}"
    );
    assert_eq!(param(d, "thickness"), "10 mm");
    assert!(
        (det(rep) - 1.0).abs() < 1e-9,
        "the frame must be right-handed, or the part is mirrored"
    );
    let holes = rep["details"]["holes"].as_array().unwrap();
    assert_eq!(holes.len(), 3);
    let mut seen = vec![];
    for h in holes {
        assert_eq!(h["kind"], "through");
        let dia = h["diameter"].as_f64().unwrap();
        let p = back(to_scan(
            rep,
            [h["x"].as_f64().unwrap(), h["y"].as_f64().unwrap(), 5.0],
        ));
        let want = [(3.0, 10.0), (5.0, 30.0), (8.0, 50.0)]
            .into_iter()
            .find(|(dd, _)| (dd - dia).abs() < 1e-5)
            .unwrap_or_else(|| panic!("unexpected diameter {dia}"));
        assert!(
            (p[0] - want.1).abs() < 1e-5 && (p[1] - 15.0).abs() < 1e-5,
            "Ø{dia} at {p:?}"
        );
        assert!((p[2] - 5.0).abs() < 1e-5, "mid-height maps to z = 5: {p:?}");
        seen.push(want.0);
        // The cut runs past both faces so it goes through.
        let z = h["z"].as_f64().unwrap();
        assert!(z < 0.0 && z + h["height"].as_f64().unwrap() > 10.0);
    }
    seen.sort_by(f64::total_cmp);
    assert_eq!(seen, [3.0, 5.0, 8.0]);
    // Features: the box then three cuts, each hole's radius from its own parameter.
    let f = d["features"].as_array().unwrap();
    assert_eq!(f.len(), 4);
    assert_eq!(
        (f[0]["type"].as_str(), f[0]["operation"].as_str()),
        (Some("box"), Some("new_body"))
    );
    for (i, c) in f[1..].iter().enumerate() {
        assert_eq!(
            (c["type"].as_str(), c["operation"].as_str()),
            (Some("cylinder"), Some("cut"))
        );
        assert_eq!(c["radius"], format!("hole{}_diameter / 2", i + 1));
    }
    assert_eq!(d["next_id"], 5);
}

#[test]
fn hole_block_is_a_plate_with_three_through_holes() {
    check_hole_block(&fixture("hole-block.stl"), |p| p);
}

/// Turning and moving the scan changes nothing about the part; the frame maps back to where the holes
/// were moved to.
#[test]
fn a_moved_scan_gives_the_same_part() {
    let mut m = fixture("hole-block.stl");
    let (c, s) = (0.6f64, 0.8f64); // a rotation about x, then a shift
    for p in &mut m.positions {
        let q = p.map(f64::from);
        *p = [
            q[0] + 7.0,
            c * q[1] - s * q[2] - 3.0,
            s * q[1] + c * q[2] + 11.0,
        ]
        .map(|x| x as f32);
    }
    check_hole_block(&m, |p| {
        let (x, y, z) = (p[0] - 7.0, p[1] + 3.0, p[2] - 11.0);
        [x, c * y + s * z, -s * y + c * z]
    });
}

/// A turned part: both round faces face outward, so they are bosses, and the round outline becomes
/// its bounding square. Both are reported.
#[test]
fn stepped_cylinder_reports_what_it_could_not_rebuild() {
    let r = design(&fixture("stepped-cylinder.stl"), "stepped").unwrap();
    let items = r.report["items"].as_array().unwrap();
    let codes: Vec<&str> = items.iter().map(|i| i["code"].as_str().unwrap()).collect();
    assert_eq!(
        codes.iter().filter(|c| **c == "boss").count(),
        2,
        "{codes:?}"
    );
    assert!(codes.contains(&"outline_rectangle"), "{codes:?}");
    assert_eq!(r.report["details"]["holes"].as_array().unwrap().len(), 0);
    for i in items {
        for k in ["severity", "title", "subject", "message", "code"] {
            assert!(i[k].as_str().is_some_and(|s| !s.is_empty()), "{k} in {i}");
        }
    }
}

/// One flat sheet has no second face, so no thickness: refused, not guessed.
#[test]
fn a_lone_sheet_is_refused() {
    let m = Mesh {
        positions: vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
        degenerate_dropped: 0,
    };
    let e = design(&m, "sheet").unwrap_err().to_string();
    assert!(e.contains("thickness"), "{e}");
}
