// SPDX-License-Identifier: MIT OR Apache-2.0
//! Saved fit outputs and descriptors measured through the real CLI.
#![allow(clippy::unwrap_used, missing_docs)]
use serde_json::{Value, json};
use std::process::Command;
#[test]
fn relations_json_and_refusals() {
    let dir = std::env::temp_dir().join(format!("rulercraft-measure-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("first.json");
    let b = dir.join("second.json");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_rulercraft"))
            .arg("measure")
            .arg(&a)
            .arg(&b)
            .output()
            .unwrap()
    };
    std::fs::write(
        &a,
        json!({"id":"bottom","fit":{"kind":"plane","center":[0,0,0],"normal":[0,0,-1]}})
            .to_string(),
    )
    .unwrap();
    std::fs::write(
        &b,
        json!({"id":"top","fit":{"kind":"plane","center":[0,0,20],"normal":[0,0,1]}}).to_string(),
    )
    .unwrap();
    let out = run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["first_id"], "bottom");
    assert_eq!(v["second_id"], "top");
    assert_eq!(v["distance"], 20.);
    assert_eq!(v["angle_deg"], 0.);
    assert_eq!(v["convention"], "distance_at_first_plane_center");
    std::fs::write(
        &a,
        json!({"kind":"cylinder","axis_point":[0,0,0],"axis":[0,0,1]}).to_string(),
    )
    .unwrap();
    std::fs::write(
        &b,
        json!({"kind":"cylinder","axis_point":[3,4,9],"axis":[0,0,-1]}).to_string(),
    )
    .unwrap();
    let out = run();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["distance"], 5.);
    for bad in [
        "{",
        r#"{"kind":"unknown"}"#,
        r#"{"kind":"point","position":[0,0]}"#,
        r#"{"kind":"cylinder","axis_point":[0,0,0],"axis":[0,0,0]}"#,
    ] {
        std::fs::write(&b, bad).unwrap();
        let out = run();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    std::fs::write(&b, vec![b' '; (1 << 20) + 1]).unwrap();
    let out = run();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("limit"));
    std::fs::remove_dir_all(dir).unwrap();
}
