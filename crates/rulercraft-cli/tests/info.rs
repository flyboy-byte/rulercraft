// SPDX-License-Identifier: MIT OR Apache-2.0
//! The CLI prints the import summary and keeps errors off stdout.
#![allow(clippy::unwrap_used, missing_docs)]
use std::process::Command;

#[test]
fn info_prints_summary_and_refuses_cleanly() {
    let dir = std::env::temp_dir().join(format!("rulercraft-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tri.stl");
    std::fs::write(&path, "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 3 0\nendloop\nendfacet\nendsolid\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .arg("info")
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        r#"{"format":"stl","vertices":3,"triangles":1,"degenerate_dropped":0,"bounds":{"min":[0,0,0],"max":[2,3,0]}}"#
    );
    std::fs::write(&path, "solid t\nendsolid\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .arg("info")
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no triangles"));
    std::fs::remove_dir_all(&dir).unwrap();
}
