// SPDX-License-Identifier: MIT OR Apache-2.0
//! Known-answer CLI checks for unaligned mesh deviation.
#![allow(clippy::unwrap_used, missing_docs)]
use serde_json::Value;
use std::process::Command;

fn plane(z: i32) -> String {
    format!(
        "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 {z}\n10 0 {z}\n0 10 {z}\n3 0 1 2\n"
    )
}

#[test]
fn deviation_measures_known_planes_and_bounds_output() {
    let dir = std::env::temp_dir().join(format!("rulercraft-cli-deviation-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let reference = dir.join("reference.ply");
    let above = dir.join("above.ply");
    let below = dir.join("below.ply");
    std::fs::write(&reference, plane(0)).unwrap();
    std::fs::write(&above, plane(1)).unwrap();
    std::fs::write(&below, plane(-1)).unwrap();
    let run = |measured: &std::path::Path, options: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rulercraft"))
            .arg("deviation")
            .arg(measured)
            .arg(&reference)
            .args(options)
            .output()
            .unwrap()
    };
    let output = run(&above, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        (value["total"].as_u64(), value["returned"].as_u64()),
        (Some(3), Some(3))
    );
    assert_eq!(
        (
            value["stats"]["mean"].as_f64(),
            value["stats"]["rms"].as_f64()
        ),
        (Some(1.0), Some(1.0))
    );
    assert_eq!(value["stats"]["beyond_tolerance"], 3);
    assert!(
        value["samples"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["distance"] == 1.0)
    );
    assert!(value["convention"].as_str().unwrap().contains("winding"));
    let below_output = run(
        &below,
        &["--offset", "1", "--limit", "1", "--tolerance", "1"],
    );
    let below_value: Value = serde_json::from_slice(&below_output.stdout).unwrap();
    assert_eq!(
        (
            below_value["total"].as_u64(),
            below_value["returned"].as_u64(),
            below_value["truncated"].as_bool()
        ),
        (Some(3), Some(1), Some(true))
    );
    assert_eq!(below_value["samples"][0]["vertex"], 1);
    assert_eq!(below_value["samples"][0]["distance"], -1.0);
    assert_eq!(below_value["stats"]["beyond_tolerance"], 0);
    assert_eq!(below_value["stats"]["count"], 3);
    let past = run(&above, &["--offset", "999", "--limit", "2"]);
    let past: Value = serde_json::from_slice(&past.stdout).unwrap();
    assert_eq!(
        (past["returned"].as_u64(), past["truncated"].as_bool()),
        (Some(0), Some(false))
    );
    let scaled = run(&above, &["--units", "m", "--reference-units", "m"]);
    assert!(scaled.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&scaled.stdout).unwrap()["stats"]["mean"],
        1000.0
    );
    for args in [
        &["--limit", "1025"][..],
        &["--offset", "-1"][..],
        &["--tolerance", "NaN"][..],
        &["--tolerance", "-1"][..],
        &["--align"][..],
    ] {
        let output = run(&above, args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    std::fs::remove_dir_all(dir).unwrap();
}
