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

#[test]
fn info_reads_ply_by_extension() {
    let path = std::env::temp_dir().join(format!("rulercraft-ply-{}.PLY", std::process::id()));
    std::fs::write(&path, b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n2 0 0\n0 3 0\n3 0 1 2\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .arg("info")
        .arg(&path)
        .args(["--units", "cm"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        r#"{"format":"ply","vertices":3,"triangles":1,"degenerate_dropped":0,"bounds":{"min":[0,0,0],"max":[20,30,0]}}"#
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn info_reads_obj_by_extension() {
    let path = std::env::temp_dir().join(format!("rulercraft-obj-{}.OBJ", std::process::id()));
    std::fs::write(&path, "v 0 0 0\nv 2 0 0\nv 0 3 0\nf -3 -2 -1\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .arg("info")
        .arg(&path)
        .args(["--units", "cm"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        r#"{"format":"obj","vertices":3,"triangles":1,"degenerate_dropped":0,"bounds":{"min":[0,0,0],"max":[20,30,0]}}"#
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn info_reads_threemf_and_uses_model_units() {
    let path = std::env::temp_dir().join(format!("rulercraft-3mf-{}.3mf", std::process::id()));
    let script = r#"
import sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w', compression=zipfile.ZIP_DEFLATED) as z:
    z.writestr('_rels/.rels', '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/a.model"/></Relationships>')
    z.writestr('3D/a.model', '<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" unit="centimeter"><resources><object id="1"><mesh><vertices><vertex x="0" y="0" z="0"/><vertex x="2" y="0" z="0"/><vertex x="0" y="3" z="0"/></vertices><triangles><triangle v1="0" v2="1" v3="2"/></triangles></mesh></object></resources><build><item objectid="1"/></build></model>')
"#;
    assert!(
        Command::new("python3")
            .args(["-c", script])
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .arg("info")
        .arg(&path)
        .args(["--units", "m"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["format"], "3mf");
    assert_eq!(v["vertices"], 3);
    assert_eq!(v["triangles"], 1);
    assert_eq!(v["bounds"]["max"], serde_json::json!([20, 30, 0]));
    std::fs::remove_file(path).unwrap();
}
