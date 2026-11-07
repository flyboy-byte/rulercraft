// SPDX-License-Identifier: MIT OR Apache-2.0
//! Committed design answers, exercised through the real CLI and engine relations.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{
    ImportLimits, Unit,
    graph::MeshGraph,
    measure::{Element, relate},
    read_mesh,
};
use serde_json::Value;
use std::{collections::HashMap, path::Path, process::Command};
fn cli(args: &[String]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_rulercraft"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn xyz(v: &Value) -> [f64; 3] {
    [
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    ]
}
fn element(v: &Value) -> Element {
    match v["kind"].as_str().unwrap() {
        "plane" => Element::Plane {
            center: xyz(&v["center"]),
            normal: xyz(&v["normal"]),
        },
        "sphere" => Element::Sphere {
            center: xyz(&v["center"]),
            radius: v["radius"].as_f64().unwrap(),
        },
        "cylinder" => Element::Cylinder {
            point: xyz(&v["axis_point"]),
            axis: xyz(&v["axis"]),
            radius: v["radius"].as_f64().unwrap(),
        },
        _ => unreachable!(),
    }
}
#[test]
fn known_parts_recover_all_stated_dimensions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/known");
    for name in [
        "hole-block",
        "stepped-cylinder",
        "sphere-seat",
        "parallel-bracket",
    ] {
        let answers: Value =
            serde_json::from_slice(&std::fs::read(root.join(format!("{name}.json"))).unwrap())
                .unwrap();
        let path = root.join(format!("{name}.stl"));
        let path = path.to_str().unwrap().to_string();
        let mesh = read_mesh(
            &path,
            &std::fs::read(&path).unwrap(),
            Unit::Mm,
            ImportLimits::default(),
        )
        .unwrap();
        let g = MeshGraph::build(&mesh);
        assert!(g.boundary_loops().is_empty(), "{name}: open boundary");
        assert_eq!(g.non_manifold_edges(), 0, "{name}");
        let bounds = cli(&["info".into(), path.clone()]);
        assert_eq!(
            xyz(&bounds["bounds"]["min"]),
            xyz(&answers["bounds"][0]),
            "{name}"
        );
        assert_eq!(
            xyz(&bounds["bounds"]["max"]),
            xyz(&answers["bounds"][1]),
            "{name}"
        );
        let tolerance = answers["tolerance_mm"].as_f64().unwrap();
        let mut fitted = HashMap::new();
        for recipe in answers["fits"].as_array().unwrap() {
            let id = recipe["id"].as_str().unwrap();
            let mut args = vec![
                "fit".into(),
                recipe["kind"].as_str().unwrap().into(),
                path.clone(),
                "--sigma".into(),
                "0".into(),
            ];
            if let Some(bounds) = recipe["box"].as_array() {
                args.push("--box".into());
                for p in bounds {
                    for v in p.as_array().unwrap() {
                        args.push(v.to_string());
                    }
                }
            }
            if let Some(near) = recipe["near"].as_array() {
                args.push("--near".into());
                for v in near {
                    args.push(v.to_string());
                }
            }
            let output = cli(&args);
            let fit = output["fit"].clone();
            for (field, expected) in recipe["expected"].as_object().unwrap() {
                let pointer = format!("/{}", field.replace('.', "/"));
                let actual = fit.pointer(&pointer).unwrap().as_f64().unwrap();
                let expected = expected.as_f64().unwrap();
                assert!(
                    (actual - expected).abs() <= tolerance,
                    "{name}/{id}/{field}: {actual} expected {expected}, tolerance {tolerance}"
                );
            }
            fitted.insert(id.to_string(), element(&fit));
        }
        for pair in answers["relations"].as_array().unwrap() {
            let first = pair["first"].as_str().unwrap();
            let second = pair["second"].as_str().unwrap();
            let r = relate(&fitted[first], &fitted[second]);
            assert!(
                (r.distance.unwrap() - pair["distance"].as_f64().unwrap()).abs() <= tolerance,
                "{name}/{first}/{second}: {r:?}"
            );
        }
    }
}
