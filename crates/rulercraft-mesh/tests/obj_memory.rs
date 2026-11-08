// SPDX-License-Identifier: MIT OR Apache-2.0
//! Regression for bounded rejection of malformed OBJ face corners.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, missing_docs)]

use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
use std::process::Command;

#[test]
fn malformed_corner_stays_within_memory_limit() {
    const CHILD: &str = "RULERCRAFT_OBJ_MEMORY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let input = format!("v 0 0 0\nf 1{}\n", "/".repeat(8_000_000));
        let error = read_mesh(
            "bad.obj",
            input.as_bytes(),
            Unit::Mm,
            ImportLimits::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("line 2: malformed face corner"), "{error}");
    } else {
        // 96 MiB accommodates the input and parser buffers, but not one allocation
        // per slash. Run separately so the limit cannot affect the test runner.
        let output = Command::new("sh")
            .args(["-c", "ulimit -c 0; ulimit -v 98304; exec \"$@\"", "sh"])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "malformed_corner_stays_within_memory_limit"])
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
