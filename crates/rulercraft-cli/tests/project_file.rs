// SPDX-License-Identifier: MIT OR Apache-2.0
//! Save/reopen a known design through the executable.
#![allow(clippy::unwrap_used)]
use serde_json::Value;
use std::{fs, process::Command};
#[test]
fn project_roundtrip_and_failed_edit_preserves_file() {
    let dir = std::env::temp_dir().join(format!("rulercraft-project-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let mesh = dir.join("top.obj");
    let file = dir.join("part.json");
    fs::write(&mesh, "v 0 0 2\nv 6 0 2\nv 0 6 2\nf 1 2 3\n").unwrap();
    let cli = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rulercraft"))
            .args(args)
            .output()
            .unwrap()
    };
    let path = file.to_str().unwrap();
    assert!(
        cli(&[
            "project",
            "create",
            path,
            mesh.to_str().unwrap(),
            "--units",
            "cm"
        ])
        .status
        .success()
    );
    fs::remove_file(mesh).unwrap(); // embedded source, no external path needed
    let fitted = cli(&["project", "fit", path, "top", "plane", "--sigma", "0"]);
    assert!(
        fitted.status.success(),
        "{}",
        String::from_utf8_lossy(&fitted.stderr)
    );
    let info = cli(&["project", "info", path]);
    let value: Value = serde_json::from_slice(&info.stdout).unwrap();
    assert_eq!(
        value["elements"][0]["last_result"]["fitted"]["Plane"]["center"][2],
        20.0
    );
    assert_eq!(value["changed"], serde_json::json!([]));
    let before = fs::read(&file).unwrap();
    for args in [
        vec!["project", "fit", path, "top", "plane"],
        vec!["project", "fit", path, "bad", "sphere"],
        vec!["project", "fit", path, "units", "plane", "--units", "mm"],
        vec!["project", "create", path, "absent.obj"],
    ] {
        let out = cli(&args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert_eq!(before, fs::read(&file).unwrap());
    }
    fs::remove_dir_all(dir).unwrap();
}
