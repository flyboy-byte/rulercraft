// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft fit` end to end: generated meshes with known answers, through the real binary.
#![allow(clippy::unwrap_used, missing_docs)]
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rulercraft-fit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn stl(triangles: &[[[f64; 3]; 3]]) -> String {
    let mut s = String::from("solid t\n");
    for t in triangles {
        s += "facet normal 0 0 0\nouter loop\n";
        for v in t {
            s += &format!("vertex {} {} {}\n", v[0], v[1], v[2]);
        }
        s += "endloop\nendfacet\n";
    }
    s + "endsolid\n"
}

/// A 20 mm cube from (0,0,0), outward winding.
fn cube() -> String {
    let c = |i: u32| {
        [
            (i & 1) as f64 * 20.0,
            ((i >> 1) & 1) as f64 * 20.0,
            ((i >> 2) & 1) as f64 * 20.0,
        ]
    };
    let mut t = vec![];
    for q in [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        t.push([c(q[0]), c(q[1]), c(q[2])]);
        t.push([c(q[0]), c(q[2]), c(q[3])]);
    }
    stl(&t)
}

/// A latitude-longitude ball of radius 12.7 at (5, -3, 2); every vertex lies on the sphere.
fn ball() -> String {
    let (r, c) = (12.7, [5.0, -3.0, 2.0]);
    let at = |i: usize, j: usize| {
        let (th, ph) = (
            std::f64::consts::PI * i as f64 / 12.0,
            std::f64::consts::TAU * j as f64 / 24.0,
        );
        [
            c[0] + r * th.sin() * ph.cos(),
            c[1] + r * th.sin() * ph.sin(),
            c[2] + r * th.cos(),
        ]
    };
    let mut t = vec![];
    for i in 0..12 {
        for j in 0..24 {
            let (a, b, d, e) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            if i > 0 {
                t.push([a, b, e]);
            }
            if i < 11 {
                t.push([b, d, e]);
            }
        }
    }
    stl(&t)
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn json(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

#[test]
fn plane_on_the_top_face_of_a_cube() {
    let path = scratch("cube.stl", &cube());
    let p = path.to_str().unwrap();
    let (code, out, err) = run(&[
        "fit", "plane", p, "--box", "-1", "-1", "19", "21", "21", "21",
    ]);
    assert_eq!(code, Some(0), "{err}");
    let v = json(&out);
    assert_eq!(v["selected"], 4);
    assert_eq!(v["fit"]["normal"], serde_json::json!([0.0, 0.0, 1.0]));
    assert_eq!(v["fit"]["center"][2], 20.0);
    assert_eq!(v["fit"]["sigma"], 0.0);
    // The same face in inches is 25.4 times further up.
    let (_, out, _) = run(&[
        "fit", "plane", p, "--units", "in", "--box", "-1", "-1", "500", "600", "600", "600",
    ]);
    assert_eq!(json(&out)["fit"]["center"][2], 508.0);
}

#[test]
fn sphere_on_a_ball_gives_its_diameter() {
    let path = scratch("ball.stl", &ball());
    let p = path.to_str().unwrap();
    let (code, out, err) = run(&["fit", "sphere", p, "--sigma", "0"]);
    assert_eq!(code, Some(0), "{err}");
    let f = &json(&out)["fit"];
    assert!((f["diameter"].as_f64().unwrap() - 25.4).abs() < 1e-4, "{f}");
    for (k, want) in [5.0, -3.0, 2.0].into_iter().enumerate() {
        assert!((f["center"][k].as_f64().unwrap() - want).abs() < 1e-4);
    }
    // Only the upper half, picked by facing direction, still gives the same ball.
    let (code, out, err) = run(&[
        "fit", "sphere", p, "--facing", "0", "0", "1", "--within", "80",
    ]);
    assert_eq!(code, Some(0), "{err}");
    let f = &json(&out)["fit"];
    assert!((f["diameter"].as_f64().unwrap() - 25.4).abs() < 1e-4, "{f}");
    assert!(f["cap_angle_deg"].as_f64().unwrap() < 90.0);
    let (code, out, _) = run(&["fit", "sphere", p, "--known-diameter", "25.4"]);
    assert_eq!(code, Some(0));
    assert_eq!(json(&out)["fit"]["diameter"], 25.4);
}

#[test]
fn refusals_exit_2_with_a_message_and_no_stdout() {
    let path = scratch("cube2.stl", &cube());
    let p = path.to_str().unwrap();
    for (args, needle) in [
        (
            vec!["fit", "plane", p, "--box", "0", "5", "0", "1", "1", "1"],
            "minimum is past its maximum on y",
        ),
        (vec!["fit", "cone", p], "unknown fit kind"),
        (
            vec!["fit", "plane", p, "--known-diameter", "5"],
            "only applies to a sphere or cylinder",
        ),
        (
            vec![
                "fit", "plane", p, "--box", "50", "50", "50", "60", "60", "60",
            ],
            "0 points marked",
        ),
        (vec!["fit", "plane", p, "--sigma", "5"], "--sigma must be"),
        (
            vec!["fit", "plane", p, "--box", "1", "2"],
            "--box needs 6 numbers",
        ),
    ] {
        let (code, out, err) = run(&args);
        assert_eq!(code, Some(2), "{args:?}");
        assert!(out.is_empty(), "{args:?}");
        assert!(err.contains(needle), "{args:?}: {err}");
    }
}

#[test]
fn cylinder_on_a_tube_gives_its_diameter() {
    let (r, mut t) = (4.0f64, vec![]);
    let at = |ring: usize, i: usize| {
        let a = std::f64::consts::TAU * i as f64 / 32.0;
        [10.0 + r * a.cos(), -5.0 + r * a.sin(), ring as f64 * 5.0]
    };
    for ring in 0..4 {
        for i in 0..32 {
            let (p, q, s, u) = (
                at(ring, i),
                at(ring, i + 1),
                at(ring + 1, i + 1),
                at(ring + 1, i),
            );
            t.push([p, q, s]);
            t.push([p, s, u]);
        }
    }
    let path = scratch("tube.stl", &stl(&t));
    let (code, out, err) = run(&["fit", "cylinder", path.to_str().unwrap(), "--sigma", "0"]);
    assert_eq!(code, Some(0), "{err}");
    let f = &json(&out)["fit"];
    assert!((f["diameter"].as_f64().unwrap() - 8.0).abs() < 1e-4, "{f}");
    assert!(f["axis"][2].as_f64().unwrap().abs() > 1.0 - 1e-9);
    assert!((f["length"].as_f64().unwrap() - 20.0).abs() < 1e-4, "{f}");
}

#[test]
fn within_requires_a_facing_direction() {
    let path = scratch("within-cube.stl", &cube());
    let (code, out, err) = run(&["fit", "plane", path.to_str().unwrap(), "--within", "30"]);
    assert_eq!(code, Some(2), "stdout={out} stderr={err}");
    assert!(out.is_empty());
    assert!(err.contains("--within requires --facing"), "{err}");
    // Options can occur in either order when both are present.
    let (code, _, err) = run(&[
        "fit",
        "plane",
        path.to_str().unwrap(),
        "--box",
        "-1",
        "-1",
        "19",
        "21",
        "21",
        "21",
        "--within",
        "90",
        "--facing",
        "0",
        "0",
        "1",
    ]);
    assert_eq!(code, Some(0), "{err}");
}
