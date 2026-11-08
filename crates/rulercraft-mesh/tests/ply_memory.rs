// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded handling of long PLY header lines.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
use std::process::Command;

#[test]
fn long_header_lines_stay_within_memory_limit() {
    const CHILD: &str = "RULERCRAFT_PLY_MEMORY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let input = format!("ply\nformat ascii 1.0 {}\n", "x ".repeat(8_000_000));
        let error = read_mesh(
            "bad.ply",
            input.as_bytes(),
            Unit::Mm,
            ImportLimits::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("line 2"), "{error}");
        drop(input);
        let input = format!(
            "ply\nformat ascii 1.0\ncomment {}\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            "x ".repeat(8_000_000)
        );
        let mesh = read_mesh(
            "good.ply",
            input.as_bytes(),
            Unit::Mm,
            ImportLimits::default(),
        )
        .unwrap();
        assert_eq!((mesh.positions.len(), mesh.triangles.len()), (3, 1));
    } else {
        // 96 MiB allows the input but not a vector entry for every header token.
        let output = Command::new("sh")
            .args(["-c", "ulimit -c 0; ulimit -v 98304; exec \"$@\"", "sh"])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "long_header_lines_stay_within_memory_limit"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
