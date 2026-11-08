// SPDX-License-Identifier: MIT OR Apache-2.0
//! Known-answer CLI checks for directional thickness.
#![allow(clippy::unwrap_used, missing_docs)]
use serde_json::Value;
use std::{fmt::Write, process::Command};

fn box_mesh() -> String {
    let mut positions: Vec<[f64; 3]> = (0..8)
        .map(|i| {
            [
                (i & 1) as f64 * 2.0,
                ((i >> 1) & 1) as f64 * 4.0,
                ((i >> 2) & 1) as f64 * 6.0,
            ]
        })
        .collect();
    let faces = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    let mut triangles = Vec::new();
    for corners in faces {
        let center =
            [0, 1, 2].map(|axis| corners.iter().map(|i| positions[*i][axis]).sum::<f64>() / 4.0);
        let mid = positions.len();
        positions.push(center);
        for i in 0..4 {
            triangles.push([mid, corners[i], corners[(i + 1) % 4]]);
        }
    }
    let mut ply = format!(
        "ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n",
        positions.len(),
        triangles.len()
    );
    for p in positions {
        writeln!(ply, "{} {} {}", p[0], p[1], p[2]).unwrap();
    }
    for t in triangles {
        writeln!(ply, "3 {} {} {}", t[0], t[1], t[2]).unwrap();
    }
    ply
}

#[test]
fn thickness_measures_known_box_and_bounds_output() {
    let dir = std::env::temp_dir().join(format!("rulercraft-cli-wall-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("box.ply");
    std::fs::write(&path, box_mesh()).unwrap();
    let run = |options: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rulercraft"))
            .arg("thickness")
            .arg(&path)
            .args(options)
            .output()
            .unwrap()
    };
    let selected = ["--box", "1", "2", "6", "1", "2", "6"];
    let result = run(&selected);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        (value["total"].as_u64(), value["returned"].as_u64()),
        (Some(1), Some(1))
    );
    assert!((value["samples"][0]["distance"].as_f64().unwrap() - 6.0).abs() < 1e-10);
    assert!(value["convention"].as_str().unwrap().contains("outward"));
    let miss = run(&["--box", "1", "2", "6", "1", "2", "6", "--max-distance", "1"]);
    assert!(miss.status.success());
    assert!(
        serde_json::from_slice::<Value>(&miss.stdout).unwrap()["samples"][0]["distance"].is_null()
    );
    let page = run(&["--offset", "1", "--limit", "2"]);
    let page: Value = serde_json::from_slice(&page.stdout).unwrap();
    assert_eq!(
        (
            page["total"].as_u64(),
            page["returned"].as_u64(),
            page["truncated"].as_bool()
        ),
        (Some(14), Some(2), Some(true))
    );
    let scaled = run(&[
        "--units", "m", "--box", "1000", "2000", "6000", "1000", "2000", "6000",
    ]);
    assert!(scaled.status.success());
    let scaled: Value = serde_json::from_slice(&scaled.stdout).unwrap();
    assert!((scaled["samples"][0]["distance"].as_f64().unwrap() - 6000.0).abs() < 1e-7);
    let past = run(&["--offset", "999", "--limit", "2"]);
    let past: Value = serde_json::from_slice(&past.stdout).unwrap();
    assert_eq!(
        (past["returned"].as_u64(), past["truncated"].as_bool()),
        (Some(0), Some(false))
    );
    for args in [
        &["--limit", "1025"][..],
        &["--max-distance", "1000001"][..],
        &["--max-distance", "NaN"][..],
        &["--offset", "-1"][..],
        &["--within", "30"][..],
        &["--sigma", "0"][..],
    ] {
        let out = run(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    std::fs::remove_dir_all(dir).unwrap();
}
