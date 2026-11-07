// SPDX-License-Identifier: MIT OR Apache-2.0
//! Saved recipes retain units and recompute known plane dimensions.
#![allow(clippy::unwrap_used)]
use rulercraft_mesh::Unit;
use rulercraft_mesh::commands::{FitKind, FitRequest, Fitted, Selection};
use rulercraft_mesh::fit::Clip;
use rulercraft_mesh::project::{Project, ProjectLimits};

const PLANE: &[u8] = b"v 0 0 2\nv 6 0 2\nv 0 6 2\nf 1 2 3\n";
fn recipe() -> FitRequest {
    FitRequest {
        kind: FitKind::Plane,
        selection: Selection::default(),
        clip: Clip::None,
        known_diameter: None,
    }
}
#[test]
fn embedded_units_recipes_and_refits() {
    let limits = ProjectLimits::default();
    let mut p = Project::new("plane.obj", PLANE, Unit::Cm, limits).unwrap();
    p.add("top", recipe()).unwrap();
    let Fitted::Plane(fit) = &p.elements()[0].last_result.fitted else {
        unreachable!()
    };
    // The design triangle lies on z=2 cm, so the fit is z=20 mm exactly.
    assert!((fit.center[2] - 20.0).abs() < 1e-10);
    assert!(fit.sigma < 1e-10); // exact plane; tolerance covers f64 roundoff
    let bytes = p.encode().unwrap();
    let reopened = Project::decode(&bytes, limits).unwrap();
    assert!(reopened.changed.is_empty());
    assert_eq!(reopened.project.elements(), p.elements());
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["elements"][0]["last_result"]["fitted"]["Plane"]["center"][2] = 21.0.into();
    let stale = Project::decode(&serde_json::to_vec(&json).unwrap(), limits).unwrap();
    assert_eq!(stale.changed, ["top"]);
    assert_eq!(stale.project.elements(), p.elements());
}
#[test]
fn failed_edits_versions_and_limits_refuse() {
    let limits = ProjectLimits::default();
    let mut p = Project::new("plane.obj", PLANE, Unit::Mm, limits).unwrap();
    let before = p.encode().unwrap();
    let mut bad = recipe();
    bad.selection.near = Some(([0.0; 3], -1.0));
    assert!(p.add("bad", bad).is_err());
    assert_eq!(before, p.encode().unwrap());
    p.add("top", recipe()).unwrap();
    assert!(
        p.add("top", recipe())
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    assert!(p.add("", recipe()).is_err());
    let bytes = p.encode().unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["version"] = 99.into();
    assert!(
        Project::decode(&serde_json::to_vec(&json).unwrap(), limits)
            .unwrap_err()
            .to_string()
            .contains("99")
    );
    assert!(Project::decode(b"{", limits).is_err());
    for tight in [
        ProjectLimits {
            max_bytes: 1,
            ..limits
        },
        ProjectLimits {
            max_elements: 0,
            ..limits
        },
    ] {
        assert!(Project::decode(&bytes, tight).is_err());
    }
    let mut tight = limits;
    tight.import.max_bytes = 1;
    assert!(Project::new("plane.obj", PLANE, Unit::Mm, tight).is_err());
}

#[test]
fn curved_recipes_preserve_settings_and_design_dimensions() {
    let limits = ProjectLimits::default();
    let ball=b"v 10 0 0\nv -10 0 0\nv 0 10 0\nv 0 -10 0\nv 0 0 10\nv 0 0 -10\nf 1 3 5\nf 3 2 5\nf 2 4 5\nf 4 1 5\nf 3 1 6\nf 2 3 6\nf 4 2 6\nf 1 4 6\n";
    let mut p = Project::new("ball.obj", ball, Unit::Mm, limits).unwrap();
    let mut r = recipe();
    r.kind = FitKind::Sphere;
    r.clip = Clip::Sigma(2);
    r.known_diameter = Some(20.0);
    r.selection.in_box = Some(([-11.0; 3], [11.0; 3]));
    r.selection.near = Some(([0.0; 3], 10.1));
    p.add("ball", r.clone()).unwrap();
    let opened = Project::decode(&p.encode().unwrap(), limits).unwrap();
    assert!(opened.changed.is_empty());
    assert_eq!(opened.project.elements()[0].recipe, r);
    let Fitted::Sphere(f) = &opened.project.elements()[0].last_result.fitted else {
        unreachable!()
    };
    assert!((f.radius - 10.0).abs() < 1e-10); // exact integer octahedron, f64 roundoff

    let mut tube = String::new();
    for z in [0.0, 10.0, 20.0] {
        for j in 0..16 {
            let a = std::f64::consts::TAU * f64::from(j) / 16.0;
            tube += &format!("v {} {} {z}\n", 10.0 * a.cos(), 10.0 * a.sin());
        }
    }
    for ring in 0..2 {
        for j in 0..16 {
            let a = ring * 16 + j + 1;
            let b = ring * 16 + (j + 1) % 16 + 1;
            tube += &format!("f {a} {b} {} {}\n", b + 16, a + 16);
        }
    }
    let mut p = Project::new("tube.obj", tube.as_bytes(), Unit::Mm, limits).unwrap();
    let mut r = recipe();
    r.kind = FitKind::Cylinder;
    r.known_diameter = Some(20.0);
    p.add("tube", r).unwrap();
    let opened = Project::decode(&p.encode().unwrap(), limits).unwrap();
    assert!(opened.changed.is_empty());
    let Fitted::Cylinder(f) = &opened.project.elements()[0].last_result.fitted else {
        unreachable!()
    };
    assert!((f.radius - 10.0).abs() < 1e-5 && (f.length - 20.0).abs() < 1e-5); // f32 source quantization
}

#[test]
fn corrupt_project_bytes_refuse_or_refit_without_panicking() {
    let limits = ProjectLimits {
        max_bytes: 1 << 16,
        max_elements: 4,
        import: rulercraft_mesh::ImportLimits {
            max_bytes: 4096,
            max_vertices: 32,
            max_triangles: 32,
        },
    };
    let mut project = Project::new("plane.obj", PLANE, Unit::Mm, limits).unwrap();
    project.add("top", recipe()).unwrap();
    let bytes = project.encode().unwrap();
    for end in 0..bytes.len() {
        let _ = Project::decode(&bytes[..end], limits);
    }
    for i in 0..bytes.len() {
        let mut corrupted = bytes.clone();
        corrupted[i] = 0xff;
        let _ = Project::decode(&corrupted, limits);
    }
}
