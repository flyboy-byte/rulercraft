// SPDX-License-Identifier: MIT OR Apache-2.0
//! The CLI version comes from its package metadata.
#![allow(clippy::unwrap_used, missing_docs)]
use std::process::Command;

#[test]
fn version_flags_report_the_package_version() {
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{flag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("rulercraft {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert!(output.stderr.is_empty());
    }
}
