// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft fit` and `measure` on point clouds through the real binary.
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

/// Two parallel planes 6 mm apart (z = 0 and z = 6), 10 x 10 points each, as an XYZ file.
fn two_planes() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("rc-cloud-{}.xyz", std::process::id()));
    let mut text = String::from("X Y Z\n");
    for z in [0.0, 6.0] {
        for i in 0..10 {
            for j in 0..10 {
                text.push_str(&format!("{} {} {z}\n", f64::from(i), f64::from(j)));
            }
        }
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn fit_and_measure_read_a_point_cloud() {
    let path = two_planes();
    let p = path.to_str().unwrap();
    let (code, out, err) = run(&[
        "fit", "plane", p, "--box", "-1", "-1", "-1", "11", "11", "1",
    ]);
    assert_eq!(code, Some(0), "{err}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["selected"], 100);
    let (code, out, err) = run(&[
        "measure", p, "plane", "--box", "-1", "-1", "-1", "11", "11", "1", "--and", "plane",
        "--box", "-1", "-1", "5", "11", "11", "7",
    ]);
    assert_eq!(code, Some(0), "{err}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let d = v["relation"]["distance"].as_f64().unwrap();
    assert!((d - 6.0).abs() < 1e-9, "{v}");
    // A cloud has no normals, and segmenting needs triangles: both refused plainly.
    let (code, _, err) = run(&["fit", "plane", p, "--facing", "0", "0", "1"]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("point cloud"), "{err}");
    let (code, _, err) = run(&["segment", p]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("`fit` and `measure`"), "{err}");
    std::fs::remove_file(&path).unwrap();
}
