// SPDX-License-Identifier: MIT OR Apache-2.0
//! cad.check: how much of a mesh the fitted planes and cylinders explain.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::cad::check;
use rulercraft_mesh::fit::{CylinderFit, PlaneFit};

type V = [f64; 3];

fn base(center: V, normal: V, u: V, v: V) -> PlaneFit {
    PlaneFit {
        center,
        normal,
        basis_u: u,
        basis_v: v,
        span_u: 40.0,
        span_v: 20.0,
        sigma: 0.0,
        form_error: 0.0,
        min_distance: 0.0,
        max_distance: 0.0,
        used: 1,
        offered: 1,
        rounds: 1,
    }
}
fn hole(point: V, axis: V) -> CylinderFit {
    CylinderFit {
        axis_point: point,
        axis,
        radius: 3.0,
        diameter: 6.0,
        length: 4.0,
        coverage_deg: 360.0,
        sigma: 0.0,
        form_error: 0.0,
        min_residual: 0.0,
        max_residual: 0.0,
        used: 1,
        offered: 1,
        rounds: 1,
        assumed_diameter: None,
        extend_before: 0.0,
        extend_after: 0.0,
    }
}
fn mesh(points: &[V]) -> Mesh {
    Mesh {
        positions: points.iter().map(|p| p.map(|x| x as f32)).collect(),
        triangles: vec![],
        degenerate_dropped: 0,
    }
}
/// 20 points on the plate (z = `z`), 6 on the hole wall at heights 1..3, 3 strays 8 mm up.
fn plate_points(z: f64) -> Vec<V> {
    let mut p: Vec<V> = (0..20)
        .map(|i| {
            [
                f64::from(i % 5) * 8.0 + 2.0,
                f64::from(i / 5) * 5.0 + 1.0,
                z,
            ]
        })
        .collect();
    for h in [1.0, 2.0, 3.0] {
        p.push([13.0, 5.0, h]); // radius 3 from the axis through (10, 5)
        p.push([7.0, 5.0, h]);
    }
    p.extend([[5.0, 5.0, 8.0], [20.0, 10.0, 8.0], [30.0, 15.0, 8.0]]);
    p
}
type Features = (Vec<(String, PlaneFit)>, Vec<(String, CylinderFit)>);
fn features() -> Features {
    (
        vec![(
            "base".into(),
            base([20., 10., 0.], [0., 0., 1.], [1., 0., 0.], [0., 1., 0.]),
        )],
        vec![("hole".into(), hole([10., 5., 2.], [0., 0., 1.]))],
    )
}

#[test]
fn a_plate_with_a_hole_is_explained_except_for_the_strays() {
    let (p, c) = features();
    let r = check(&mesh(&plate_points(0.0)), &p, &c, 0.05).unwrap();
    assert_eq!((r.vertices, r.explained), (29, 26));
    assert_eq!(r.unexplained, vec![26, 27, 28]);
    assert!(r.rms < 1e-6 && r.max < 1e-6);
    assert!((r.explained_fraction - 26.0 / 29.0).abs() < 1e-12);
    assert_eq!(
        r.per_feature,
        vec![("base".to_string(), 20), ("hole".to_string(), 6)]
    );
}

#[test]
fn the_tolerance_decides_what_is_explained() {
    let (p, c) = features();
    let m = mesh(&plate_points(0.03)); // the plate points sit 0.03 off the fitted plane
    let loose = check(&m, &p, &c, 0.05).unwrap();
    assert_eq!(loose.explained, 26);
    assert!((loose.max - 0.03).abs() < 1e-5 && loose.rms > 0.01);
    let tight = check(&m, &p, &c, 0.02).unwrap();
    assert_eq!(tight.explained, 6); // only the hole wall
}

#[test]
fn a_cylinder_does_not_explain_points_beyond_its_length() {
    let (p, c) = features();
    let far = mesh(&[[13.0, 5.0, 50.0], [13.0, 5.0, 2.0]]);
    let r = check(&far, &p[..0], &c, 0.05).unwrap();
    assert_eq!((r.explained, r.unexplained), (1, vec![0]));
}

#[test]
fn turning_and_moving_everything_changes_nothing() {
    let (a, t) = (0.8f64, [40.0, -10.0, 6.0]);
    let (s, co) = a.sin_cos();
    let tr = |v: V| {
        [
            v[0] * co - v[1] * s + t[0],
            v[0] * s + v[1] * co + t[1],
            v[2] + t[2],
        ]
    };
    let rot = |v: V| [v[0] * co - v[1] * s, v[0] * s + v[1] * co, v[2]];
    let (p, c) = features();
    let p2 = vec![(
        "base".to_string(),
        base(
            tr([20., 10., 0.]),
            rot([0., 0., 1.]),
            rot([1., 0., 0.]),
            rot([0., 1., 0.]),
        ),
    )];
    let c2 = vec![(
        "hole".to_string(),
        hole(tr([10., 5., 2.]), rot([0., 0., 1.])),
    )];
    let pts = plate_points(0.0);
    let r1 = check(&mesh(&pts), &p, &c, 0.05).unwrap();
    let moved: Vec<V> = pts.iter().map(|&v| tr(v)).collect();
    let r2 = check(&mesh(&moved), &p2, &c2, 0.05).unwrap();
    assert_eq!(
        (r1.explained, &r1.unexplained, &r1.per_feature),
        (r2.explained, &r2.unexplained, &r2.per_feature)
    );
}

#[test]
fn bad_input_is_refused_or_trivial() {
    let (p, c) = features();
    for t in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(check(&mesh(&[[0.0; 3]]), &p, &c, t).is_err());
    }
    let r = check(&Mesh::default(), &p, &c, 0.1).unwrap();
    assert_eq!((r.vertices, r.explained_fraction), (0, 1.0));
    // No features at all: nothing is explained.
    let r = check(&mesh(&[[1.0, 2.0, 3.0]]), &[], &[], 0.1).unwrap();
    assert_eq!((r.explained, r.unexplained), (0, vec![0]));
}
