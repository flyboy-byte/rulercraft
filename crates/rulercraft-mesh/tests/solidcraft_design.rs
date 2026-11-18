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

/// A turned part: the upper step stands on the lower one, so it is a boss; the lower step's wall is the round
/// outline, which the plate's enclosing rectangle stands in for. Both simplifications are reported.
#[test]
fn stepped_cylinder_rebuilds_its_step_on_a_circular_base() {
    let r = design(&fixture("stepped-cylinder.stl"), "stepped").unwrap();
    let items = r.report["items"].as_array().unwrap();
    // Its round outline is traced now: a circle, nothing lost.
    assert!(items.is_empty(), "{items:?}");
    assert_eq!(
        feature(&r.design, "sketch")["sketch"]["curves"][0]["type"],
        "circle"
    );
    let bosses = r.report["details"]["bosses"].as_array().unwrap();
    assert_eq!(bosses.len(), 1);
    assert!((bosses[0]["diameter"].as_f64().unwrap() - 12.0).abs() < 1e-5);
    assert!((bosses[0]["standing_height"].as_f64().unwrap() - 10.0).abs() < 1e-5);
    assert_eq!(param(&r.design, "thickness"), "10 mm");
    for i in items {
        for k in ["severity", "title", "subject", "message", "code"] {
            assert!(i[k].as_str().is_some_and(|s| !s.is_empty()), "{k} in {i}");
        }
    }
}

/// fixtures/known/bracket: an 80 x 50 x 8 plate with a Ø10 x 6 boss at (20, 25), a 20 x 15 x 3 pocket from
/// (45, 10), a Ø6 through hole at (70, 40) and a Ø4 hole blind from the top to z = 3 at (10, 10), made by
/// SolidCraft from bracket.solidcraft.json. Every feature maps back to where it was designed.
fn check_bracket(mesh: &Mesh, back: impl Fn([f64; 3]) -> [f64; 3]) {
    let r = design(mesh, "bracket").unwrap();
    let (d, rep) = (&r.design, &r.report);
    assert_eq!(
        rep["items"].as_array().unwrap().len(),
        0,
        "nothing lost: {}",
        rep["items"]
    );
    let (l, w) = (param(d, "length"), param(d, "width"));
    assert!(
        (l == "80 mm" && w == "50 mm") || (l == "50 mm" && w == "80 mm"),
        "{l} x {w}"
    );
    assert_eq!(param(d, "thickness"), "8 mm");
    assert!((det(rep) - 1.0).abs() < 1e-9);
    let close = |p: [f64; 3], q: [f64; 3]| (0..3).all(|k| (p[k] - q[k]).abs() < 1e-4);
    let det_ = &rep["details"];
    let at = |v: &Value, z: f64| {
        back(to_scan(
            rep,
            [v["x"].as_f64().unwrap(), v["y"].as_f64().unwrap(), z],
        ))
    };
    let bosses = det_["bosses"].as_array().unwrap();
    assert_eq!(bosses.len(), 1);
    assert!(
        close(at(&bosses[0], 8.0), [20.0, 25.0, 8.0]),
        "boss at {:?}",
        at(&bosses[0], 8.0)
    );
    assert_eq!(param(d, "boss1_diameter"), "10 mm");
    assert_eq!(param(d, "boss1_height"), "6 mm");
    let pockets = det_["pockets"].as_array().unwrap();
    assert_eq!(pockets.len(), 1);
    assert_eq!(param(d, "pocket1_depth"), "3 mm");
    // The pocket's rectangle, mapped back, spans x 45..65 and y 10..25 at z = 5 whichever way the frame turned.
    let p = &pockets[0];
    let (x0, y0) = (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap());
    let (pl, pw) = (p["length"].as_f64().unwrap(), p["width"].as_f64().unwrap());
    let corners: Vec<[f64; 3]> = [(0.0, 0.0), (pl, 0.0), (0.0, pw), (pl, pw)]
        .iter()
        .map(|(a, b)| back(to_scan(rep, [x0 + a, y0 + b, 5.0])))
        .collect();
    for want in [
        [45.0, 10.0, 5.0],
        [65.0, 10.0, 5.0],
        [45.0, 25.0, 5.0],
        [65.0, 25.0, 5.0],
    ] {
        assert!(
            corners.iter().any(|c| close(*c, want)),
            "pocket corner {want:?} in {corners:?}"
        );
    }
    let holes = det_["holes"].as_array().unwrap();
    assert_eq!(holes.len(), 2);
    for h in holes {
        let dia = h["diameter"].as_f64().unwrap();
        if (dia - 6.0).abs() < 1e-5 {
            assert_eq!(h["kind"], "through");
            assert!(close(at(h, 4.0), [70.0, 40.0, 4.0]));
        } else {
            assert!((dia - 4.0).abs() < 1e-5, "Ø{dia}");
            assert_eq!(h["kind"], "blind from above");
            assert!((h["from"].as_f64().unwrap() - 3.0).abs() < 1e-5);
            assert!(close(at(h, 4.0), [10.0, 10.0, 4.0]));
        }
    }
    // Build order: plate, boss joined, pocket and holes cut.
    let ops: Vec<(&str, &str)> = d["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["type"].as_str().unwrap(),
                f["operation"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ops,
        [
            ("box", "new_body"),
            ("cylinder", "join"),
            ("box", "cut"),
            ("cylinder", "cut"),
            ("cylinder", "cut")
        ]
    );
}

#[test]
fn bracket_rebuilds_boss_pocket_and_holes() {
    check_bracket(&fixture("bracket.stl"), |p| p);
}

/// Turned 30° in its own plane and tilted and moved: the outline's rectangle still lines up with its edges.
#[test]
fn a_turned_bracket_gives_the_same_part() {
    let mut m = fixture("bracket.stl");
    let (cz, sz) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let (c, s) = (0.6f64, 0.8f64);
    for p in &mut m.positions {
        let q = p.map(f64::from);
        let r = [cz * q[0] - sz * q[1], sz * q[0] + cz * q[1], q[2]];
        *p = [
            r[0] + 7.0,
            c * r[1] - s * r[2] - 3.0,
            s * r[1] + c * r[2] + 11.0,
        ]
        .map(|x| x as f32);
    }
    check_bracket(&m, |p| {
        let (x, y, z) = (p[0] - 7.0, p[1] + 3.0, p[2] - 11.0);
        let r = [x, c * y + s * z, -s * y + c * z];
        [cz * r[0] + sz * r[1], -sz * r[0] + cz * r[1], r[2]]
    });
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

fn feature<'a>(d: &'a Value, ty: &str) -> &'a Value {
    d["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["type"] == ty)
        .unwrap_or_else(|| panic!("no {ty} feature"))
}

/// Shoelace area of a sketch's line loop, mm².
fn sketch_area(d: &Value) -> (usize, f64) {
    let pts: Vec<[f64; 2]> = feature(d, "sketch")["sketch"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["id"] != "origin")
        .map(|p| [p["pos"][0].as_f64().unwrap(), p["pos"][1].as_f64().unwrap()])
        .collect();
    let n = pts.len();
    let a: f64 = (0..n)
        .map(|i| pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1])
        .sum();
    (n, a / 2.0)
}

/// SolidCraft built the L (60 x 20 plus 20 x 20, 8 mm thick, Ø6 hole at (10, 30)) as a sketch extruded:
/// 12,573.805 mm³. The rebuild must trace the six corners, not box the part.
#[test]
fn an_l_plate_keeps_its_outline() {
    let r = design(&fixture("l-plate.stl"), "l-plate").unwrap();
    let (n, area) = sketch_area(&r.design);
    assert_eq!(n, 6, "corners");
    assert!((area - 1600.0).abs() < 0.5, "outline area {area}");
    assert_eq!(feature(&r.design, "extrude")["operation"], "new_body");
    assert_eq!(param(&r.design, "thickness"), "8 mm");
    let h = &r.report["details"]["holes"][0];
    assert!((h["diameter"].as_f64().unwrap() - 6.0).abs() < 0.01);
    let codes: Vec<&str> = r.report["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap())
        .collect();
    assert!(codes.is_empty(), "nothing lost: {codes:?}");
    // The hole sits in the 20 mm arm: 10 mm from its two outer edges, whichever way the frame turned.
    let at = [h["x"].as_f64().unwrap(), h["y"].as_f64().unwrap()];
    let p = sketch_points(&r.design);
    let inside = point_in(at, &p);
    assert!(inside, "hole at {at:?} is inside the traced outline");
}

fn sketch_points(d: &Value) -> Vec<[f64; 2]> {
    feature(d, "sketch")["sketch"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["id"] != "origin")
        .map(|p| [p["pos"][0].as_f64().unwrap(), p["pos"][1].as_f64().unwrap()])
        .collect()
}

fn point_in(q: [f64; 2], p: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        if (a[1] > q[1]) != (b[1] > q[1])
            && q[0] < (b[0] - a[0]) * (q[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

/// A Ø50 x 6 round plate with a Ø8 hole 10 mm off centre (SolidCraft: 11,479.380 mm³) is a circle sketch.
#[test]
fn a_round_plate_is_a_circle() {
    let r = design(&fixture("round-plate.stl"), "round-plate").unwrap();
    let c = &feature(&r.design, "sketch")["sketch"]["curves"][0];
    assert_eq!(c["type"], "circle");
    assert!((c["r"].as_f64().unwrap() - 25.0).abs() < 0.01, "{c}");
    assert!(r.report["items"].as_array().unwrap().is_empty());
    let h = &r.report["details"]["holes"][0];
    let centre = &feature(&r.design, "sketch")["sketch"]["points"][1]["pos"];
    let d = (h["x"].as_f64().unwrap() - centre[0].as_f64().unwrap())
        .hypot(h["y"].as_f64().unwrap() - centre[1].as_f64().unwrap());
    assert!((d - 10.0).abs() < 0.01, "hole {d} mm from the centre");
}

/// A rectangle still builds as a box: tracing must not turn every plate into a sketch.
#[test]
fn a_plain_plate_stays_a_box() {
    let r = design(&fixture("hole-block.stl"), "hole-block").unwrap();
    assert_eq!(r.design["features"][0]["type"], "box");
}

/// A 40 x 30 x 10 plate with a Ø6 hole and a Ø12 x 4 counterbore on the same axis (SolidCraft: 11,377.965 mm³).
/// The two coaxial rounds rebuild as two cuts at one position, the wide one blind from the top.
#[test]
fn a_counterbore_is_two_coaxial_holes() {
    let r = design(&fixture("counterbore.stl"), "counterbore").unwrap();
    assert!(r.report["items"].as_array().unwrap().is_empty());
    let holes = r.report["details"]["holes"].as_array().unwrap();
    assert_eq!(holes.len(), 2);
    let mut dia: Vec<f64> = holes
        .iter()
        .map(|h| h["diameter"].as_f64().unwrap())
        .collect();
    dia.sort_by(f64::total_cmp);
    assert!(
        (dia[0] - 6.0).abs() < 0.01 && (dia[1] - 12.0).abs() < 0.01,
        "{dia:?}"
    );
    let (a, b) = (&holes[0], &holes[1]);
    let d = (a["x"].as_f64().unwrap() - b["x"].as_f64().unwrap())
        .hypot(a["y"].as_f64().unwrap() - b["y"].as_f64().unwrap());
    assert!(d < 0.01, "axes {d} mm apart");
    let kinds: Vec<&str> = holes.iter().map(|h| h["kind"].as_str().unwrap()).collect();
    assert!(
        kinds.contains(&"through") && kinds.iter().any(|k| k.starts_with("blind")),
        "{kinds:?}"
    );
}

/// A 50 x 30 x 6 plate with a 12 x 10 window cut through it (SolidCraft: 8,280 mm³). The window is an inner
/// loop of both faces that is not a circle, so it rebuilds as a cut sketch, and nothing is reported lost.
#[test]
fn a_window_is_a_cut_through_sketch() {
    let r = design(&fixture("window.stl"), "window").unwrap();
    assert!(
        r.report["items"].as_array().unwrap().is_empty(),
        "{}",
        r.report["items"]
    );
    let cut = &r.report["details"]["cutouts"][0];
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in cut.as_array().unwrap() {
        for k in 0..2 {
            let v = p[k].as_f64().unwrap();
            lo[k] = lo[k].min(v);
            hi[k] = hi[k].max(v);
        }
    }
    let mut size = [hi[0] - lo[0], hi[1] - lo[1]];
    size.sort_by(f64::total_cmp);
    assert!(
        (size[0] - 10.0).abs() < 0.01 && (size[1] - 12.0).abs() < 0.01,
        "{size:?}"
    );
    let cutout = r.design["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "Cutout1")
        .unwrap();
    assert_eq!(cutout["operation"], "cut");
}

/// Editing `thickness` in SolidCraft must keep the part whole: holes that open on the top reach it and a boss on the
/// top face stays on it. Checked in SolidCraft: at 12 mm the bracket's Ø6 hole wall is 12 mm long and its boss
/// still stands 6 mm.
#[test]
fn holes_and_bosses_follow_the_thickness() {
    let r = design(&fixture("bracket.stl"), "bracket").unwrap();
    let by = |n: &str| {
        r.design["features"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == n)
            .unwrap_or_else(|| panic!("no {n}"))
            .clone()
    };
    let hole = by("Hole1");
    let (kind, height) = (
        &r.report["details"]["holes"][0]["kind"],
        hole["height"].as_str(),
    );
    assert_eq!(kind, "through");
    assert!(
        height.is_some_and(|h| h.starts_with("thickness + ")),
        "{height:?}"
    );
    assert!(
        by("Boss1")["base"][2]
            .as_str()
            .is_some_and(|z| z.starts_with("thickness - "))

/// The import report measures the scan against the rebuilt part. Rebuilt from SolidCraft's own part, the scan
/// lies on it (to float precision); one vertex pulled 2 mm off the top face is found, and only that one.
#[test]
fn the_check_finds_a_bump() {
    let clean = design(&fixture("hole-block.stl"), "hole-block").unwrap();
    let c = &clean.report["details"]["check"];
    assert!(c["max_mm"].as_f64().unwrap() < 1e-3, "{c}");
    assert_eq!(c["within_tolerance"].as_f64().unwrap(), 1.0);

    let mut m = fixture("hole-block.stl");
    // A vertex in the middle of the top face: the highest point nearest the centre of the scan.
    let top = m.positions.iter().map(|p| p[2]).fold(f32::MIN, f32::max);
    let (cx, cy) = (
        m.positions.iter().map(|p| p[0]).sum::<f32>() / m.positions.len() as f32,
        m.positions.iter().map(|p| p[1]).sum::<f32>() / m.positions.len() as f32,
    );
    let i = (0..m.positions.len())
        .filter(|&i| (m.positions[i][2] - top).abs() < 1e-4)
        .min_by(|&a, &b| {
            let d = |i: usize| (m.positions[i][0] - cx).hypot(m.positions[i][1] - cy);
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    m.positions[i][2] += 2.0;
    let r = design(&m, "bumped").unwrap();
    let c = &r.report["details"]["check"];
    assert!((c["max_mm"].as_f64().unwrap() - 2.0).abs() < 0.05, "{c}");
    assert!(
        c["p95_mm"].as_f64().unwrap() < 1e-3,
        "one point is not 5 %: {c}"
    );
}
