// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft solidcraft` end to end through the real binary (docs/specs/mesh-to-cad.md).
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

fn fixture(name: &str) -> String {
    format!("{}/../../fixtures/known/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn writes_the_design_once_and_prints_the_report() {
    let dir = std::env::temp_dir().join(format!("rulercraft-sc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("hole-block.json");
    let _ = std::fs::remove_file(&out);
    let m = fixture("hole-block.stl");
    let (code, stdout, err) = run(&["solidcraft", &m, "--out", out.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{err}");
    let j: Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        j["report"]["summary"]
            .as_str()
            .unwrap()
            .starts_with("A 60 x 30 x 10 mm plate with 3 holes")
    );
    let d: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(d["features"].as_array().unwrap().len(), 4);
    // Never over an existing file.
    let (code, _, err) = run(&["solidcraft", &m, "--out", out.to_str().unwrap()]);
    assert_eq!(code, Some(2));
    assert!(err.contains("exists"), "{err}");
    // Without --out both are printed; --units scales the part.
    let (code, stdout, _) = run(&["solidcraft", &m, "--units", "cm"]);
    assert_eq!(code, Some(0));
    let j: Value = serde_json::from_str(&stdout).unwrap();
    let thickness = j["design"]["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "thickness")
        .unwrap()["expr"]
        .clone();
    assert_eq!(thickness, "100 mm");
    let (code, _, err) = run(&["solidcraft", &m, "--bogus"]);
    assert_eq!(code, Some(2));
    assert!(err.contains("unknown option"), "{err}");
}
