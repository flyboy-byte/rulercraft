// SPDX-License-Identifier: MIT OR Apache-2.0
//! Large declared headers remain practical and duplicate declarations still refuse.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
use std::fmt::Write;

#[test]
fn many_distinct_elements_and_properties_preserve_validation() {
    let mut header = String::from("ply\nformat ascii 1.0\n");
    for i in 0..20_000 {
        writeln!(header, "element extra{i} 0").unwrap();
    }
    header.push_str("element metadata 0\n");
    for i in 0..20_000 {
        writeln!(header, "property float field{i}").unwrap();
    }
    // A property name may be reused on a different element.
    header.push_str("property float x\n");
    let mut valid = header.clone();
    valid.push_str("element vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n");
    let mesh = read_mesh(
        "many.ply",
        valid.as_bytes(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    assert_eq!((mesh.positions.len(), mesh.triangles.len()), (3, 1));
    for (suffix, expected) in [
        ("element extra3 0\n", "duplicate element"),
        ("property float field3\n", "duplicate property"),
    ] {
        let invalid = format!("{header}{suffix}");
        let error = read_mesh(
            "bad.ply",
            invalid.as_bytes(),
            Unit::Mm,
            ImportLimits::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(expected), "{error}");
    }
}
