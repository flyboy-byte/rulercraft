// SPDX-License-Identifier: MIT OR Apache-2.0
//! Runs the viewer's geometry tests (ray picking, brush, camera maths) with Node when it is installed.
#![allow(clippy::unwrap_used, missing_docs)]
use std::process::Command;

#[test]
fn viewer_geometry_passes_under_node() {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/geom.test.js");
    match Command::new("node").arg(script).output() {
        Err(_) => {
            eprintln!("skipped: node is not installed, so the viewer geometry tests did not run")
        }
        Ok(out) => assert!(
            out.status.success(),
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}
