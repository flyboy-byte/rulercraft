// SPDX-License-Identifier: MIT OR Apache-2.0
//! Oversized regular files refuse by byte limit before allocating their contents.
#![allow(clippy::unwrap_used)]

#[test]
#[cfg(target_os = "linux")]
fn oversized_sparse_file_reports_the_byte_limit_without_large_allocation() {
    let script = r#"
import json, pathlib, resource, subprocess, sys, tempfile
with tempfile.TemporaryDirectory(prefix='rulercraft-limit-test-') as directory:
    path = pathlib.Path(directory) / 'large.stl'
    with path.open('wb') as stream:
        stream.truncate((2 << 30) + 1)
    def constrained():
        resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    result = subprocess.run([sys.argv[1], 'info', str(path)], capture_output=True,
                            text=True, timeout=5, preexec_fn=constrained)
    print(json.dumps({'code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}))
"#;
    let out = std::process::Command::new("python3")
        .args(["-c", script, env!("CARGO_BIN_EXE_rulercraft")])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["code"], 2);
    assert_eq!(value["stdout"], "");
    assert!(
        value["stderr"]
            .as_str()
            .unwrap()
            .contains("byte limit exceeded"),
        "{value}"
    );
}
