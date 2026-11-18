// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft measure` end to end: known design distances and angles through the real binary.
#![allow(clippy::unwrap_used, missing_docs)]
use serde_json::Value;
use std::process::Command;

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

fn ok(args: &[&str]) -> Value {
    let (code, out, err) = run(args);
    assert_eq!(code, Some(0), "{err}");
    serde_json::from_str(&out).unwrap()
}

fn fixture(name: &str) -> String {
    format!("{}/../../fixtures/known/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn args<'a>(mesh: &'a str, rest: &'a str) -> Vec<&'a str> {
    let mut v = vec!["measure", mesh];
    v.extend(rest.split_whitespace());
    v
}

/// hole-block.json: the small and medium holes' axes are 20 mm apart (design 10 and 30 on x).
#[test]
fn hole_axes_are_twenty_apart() {
    let m = fixture("hole-block.stl");
    let j = ok(&args(
        &m,
        "cylinder --box 8.4 13.4 1 11.6 16.6 9 --and cylinder --box 27.4 12.4 1 32.6 17.6 9",
    ));
    let d = j["relation"]["distance"].as_f64().unwrap();
    assert!((d - 20.0).abs() < 1e-5, "distance {d}");
    assert!(j["relation"]["angle_deg"].as_f64().unwrap() < 1e-3);
    assert!((j["first"]["fit"]["diameter"].as_f64().unwrap() - 3.0).abs() < 1e-5);
    assert_eq!(j["second"]["fit"]["kind"], "cylinder");
    assert!(j["relation"]["convention"].as_str().unwrap().len() > 10);
}

/// Opposite faces of the 60 x 30 x 10 block are 10 apart and parallel; a side face is square to the top.
#[test]
fn block_faces_parallel_and_square() {
    let m = fixture("hole-block.stl");
    // Box selections only: a side face's vertices are all corners, whose normals point diagonally, so
    // `--facing` would mark none of them.
    let top = "plane --box -1 -1 9.5 61 31 10.5";
    let bottom = "plane --box -1 -1 -0.5 61 31 0.5";
    let side = "plane --box 59.5 -1 -1 60.5 31 11";
    let j = ok(&args(&m, &format!("{top} --and {bottom}")));
    assert!(
        (j["relation"]["distance"].as_f64().unwrap() - 10.0).abs() < 1e-5,
        "{j}"
    );
    assert!(j["relation"]["angle_deg"].as_f64().unwrap() < 1e-3);
    let j = ok(&args(&m, &format!("{top} --and {side}")));
    assert!(
        (j["relation"]["angle_deg"].as_f64().unwrap() - 90.0).abs() < 1e-3,
        "{j}"
    );
    assert!(
        j["relation"]["distance"].is_null(),
        "no distance between square planes"
    );
    // --units scales both fits (selections are in mm after scaling): read as centimetres, 100 mm thick.
    let j = ok(&args(
        &m,
        "plane --box -1 -1 95 601 301 105 --units cm --and plane --box -1 -1 -5 601 301 5",
    ));
    assert!(
        (j["relation"]["distance"].as_f64().unwrap() - 100.0).abs() < 1e-4,
        "{j}"
    );
}

#[test]
fn bad_requests_are_refused() {
    let m = fixture("hole-block.stl");
    for (rest, want) in [
        ("plane", "needs two fits"),
        ("plane --and", "second fit needs a kind"),
        ("plane --and plane --and plane", "more than once"),
        ("plane --units cm --and plane --units in", "--units once"),
        ("cone --and plane", "first fit: unknown fit kind"),
        ("plane --and sphere --near 0 0 0 0.001", "second fit:"),
    ] {
        let (code, _, err) = run(&args(&m, rest));
        assert_eq!(code, Some(2), "{rest}");
        assert!(err.contains(want), "{rest}: want `{want}` in {err}");
    }
}
