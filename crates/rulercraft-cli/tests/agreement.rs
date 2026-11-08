// SPDX-License-Identifier: MIT OR Apache-2.0
//! The CLI and library must return the same measurements for the same known design.
#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]
use rulercraft_mesh::commands::{FitKind, FitRequest, Fitted, Selection, fit};
use rulercraft_mesh::fit::Clip;
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
use serde_json::Value;
use std::process::Command;

#[test]
fn known_hole_fits_agree_between_cli_and_library() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/known/hole-block.stl");
    let mesh = read_mesh(
        "hole-block.stl",
        &std::fs::read(&path).unwrap(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    let graph = MeshGraph::build(&mesh);
    for (min, max, expected_diameter) in [
        ([8.4, 13.4, 1.0], [11.6, 16.6, 9.0], 3.0),
        ([27.4, 12.4, 1.0], [32.6, 17.6, 9.0], 5.0),
        ([45.9, 10.9, 1.0], [54.1, 19.1, 9.0], 8.0),
    ] {
        let request = FitRequest {
            kind: FitKind::Cylinder,
            selection: Selection {
                in_box: Some((min, max)),
                ..Default::default()
            },
            clip: Clip::None,
            known_diameter: None,
        };
        let result = fit(&mesh, &graph, &request).unwrap();
        let Fitted::Cylinder(cylinder) = result.fitted else {
            panic!("expected cylinder")
        };
        let args: Vec<_> = min.into_iter().chain(max).map(|n| n.to_string()).collect();
        let output = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
            .arg("fit")
            .arg("cylinder")
            .arg(&path)
            .arg("--box")
            .args(&args)
            .args(["--sigma", "0"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let json: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(json["selected"], result.selected);
        let cli = &json["fit"];
        // JSON numbers round-trip f64, so identical calculations agree exactly.
        for (name, value) in [
            ("radius", cylinder.radius),
            ("diameter", cylinder.diameter),
            ("length", cylinder.length),
            ("sigma", cylinder.sigma),
        ] {
            assert_eq!(cli[name].as_f64().unwrap(), value, "{name}");
        }
        assert_eq!(cli["axis"], serde_json::json!(cylinder.axis));
        assert_eq!(cli["axis_point"], serde_json::json!(cylinder.axis_point));
        // The corpus tolerance is fixed by its design description and f32 quantization.
        assert!((cylinder.diameter - expected_diameter).abs() < 1e-5);
    }
}
