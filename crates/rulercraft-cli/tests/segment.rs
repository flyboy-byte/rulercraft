// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft segment` exposes the existing region algorithms with stable summaries.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn ply(positions: &[[f64; 3]], faces: &[[u32; 3]]) -> String {
    let mut text = format!(
        "ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n",
        positions.len(),
        faces.len()
    );
    for p in positions {
        text += &format!("{} {} {}\n", p[0], p[1], p[2]);
    }
    for f in faces {
        text += &format!("3 {} {} {}\n", f[0], f[1], f[2]);
    }
    text
}

fn cube() -> String {
    let positions = [
        [0., 0., 0.],
        [20., 0., 0.],
        [0., 20., 0.],
        [20., 20., 0.],
        [0., 0., 20.],
        [20., 0., 20.],
        [0., 20., 20.],
        [20., 20., 20.],
    ];
    let faces = [
        [0, 2, 3],
        [0, 3, 1],
        [4, 5, 7],
        [4, 7, 6],
        [0, 1, 5],
        [0, 5, 4],
        [2, 6, 7],
        [2, 7, 3],
        [0, 4, 6],
        [0, 6, 2],
        [1, 3, 7],
        [1, 7, 5],
    ];
    ply(&positions, &faces)
}

fn tube(sides: usize) -> String {
    let mut positions = Vec::new();
    for row in 0..=4 {
        let y = 15.0 * row as f64 / 4.0;
        for i in 0..sides {
            let angle = std::f64::consts::TAU * i as f64 / sides as f64;
            positions.push([3.0 + 6.0 * angle.cos(), y, -2.0 + 6.0 * angle.sin()]);
        }
    }
    let mut faces = Vec::new();
    for row in 0..4 {
        for i in 0..sides {
            let a = (row * sides + i) as u32;
            let b = (row * sides + (i + 1) % sides) as u32;
            let c = b + sides as u32;
            let d = a + sides as u32;
            faces.extend([[a, b, c], [a, c, d]]);
        }
    }
    ply(&positions, &faces)
}

fn run(path: &PathBuf, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .arg("segment")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn mesh_file(name: &str, text: &str) -> PathBuf {
    let thread = std::thread::current();
    let test = thread.name().unwrap_or("test");
    let path = std::env::temp_dir().join(format!(
        "rulercraft-segment-{}-{test}-{name}.ply",
        std::process::id()
    ));
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn default_uses_scan_cylinders_and_other_modes_are_selectable() {
    let path = mesh_file("cube", &cube());
    let (code, out, err) = run(&path, &[]);
    assert_eq!(code, Some(0), "{err}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["kind"], "cylinders");
    assert_eq!(v["regions"].as_array().unwrap().len(), 0);
    assert_eq!(v["unassigned"], 12);

    let (code, out, err) = run(&path, &["--kind", "planes"]);
    assert_eq!(code, Some(0), "{err}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["regions"].as_array().unwrap().len(), 6);
    assert!(
        v["regions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["triangle_count"] == 2)
    );

    std::fs::remove_file(path).unwrap();

    let fine = mesh_file("fine-tube", &tube(32));
    let (code, out, err) = run(&fine, &[]);
    assert_eq!(code, Some(0), "{err}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["regions"].as_array().unwrap().len(), 1);
    assert!((v["regions"][0]["fit"]["radius"].as_f64().unwrap() - 6.0).abs() < 1e-4);
    std::fs::remove_file(fine).unwrap();

    let coarse = mesh_file("coarse-tube", &tube(12));
    let (code, out, err) = run(&coarse, &["--kind", "strips"]);
    assert_eq!(code, Some(0), "{err}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["regions"].as_array().unwrap().len(), 1);
    assert_eq!(v["regions"][0]["strip_count"], 12);
    std::fs::remove_file(coarse).unwrap();
}

#[test]
fn unknown_kind_refuses_without_stdout() {
    let path = mesh_file("refusal", &cube());
    let (code, out, err) = run(&path, &["--kind", "unknown"]);
    assert_eq!(code, Some(2));
    assert!(out.is_empty());
    assert!(err.contains("planes, cylinders or strips"));
    std::fs::remove_file(path).unwrap();
}
