// SPDX-License-Identifier: MIT OR Apache-2.0
//! Optional private comparisons, using the same helper as the manual command.
#![allow(clippy::unwrap_used)]

#[test]
fn local_reference_numbers() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("tools/reference.py"))
        .arg("--binary")
        .arg(env!("CARGO_BIN_EXE_rulercraft"))
        .output()
        .unwrap();
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reference_helper_checks_real_cli_numbers() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .args([
            "-m",
            "unittest",
            "test_reference.ReferenceTests.test_real_cli_fixed_plane",
        ])
        .current_dir(root.join("tools"))
        .env("RULERCRAFT_TEST_BINARY", env!("CARGO_BIN_EXE_rulercraft"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
