// SPDX-License-Identifier: MIT OR Apache-2.0
//! cad.solidcraft checks: a plate whose design is known exactly.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::cad::{Step, recipe};
use rulercraft_mesh::fit::{CylinderFit, PlaneFit};

type V = [f64; 3];

fn plane(center: V, normal: V, u: V, v: V, su: f64, sv: f64) -> PlaneFit {
    PlaneFit {
        center,
        normal,
        basis_u: u,
        basis_v: v,
        span_u: su,
        span_v: sv,
        sigma: 0.0,
        form_error: 0.0,
        min_distance: 0.0,
        max_distance: 0.0,
        used: 1,
        offered: 1,
        rounds: 1,
    }
}
fn cyl(point: V, axis: V, d: f64, len: f64) -> CylinderFit {
    CylinderFit {
        axis_point: point,
        axis,
        radius: d / 2.0,
        diameter: d,
        length: len,
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
/// Rotates about z by `a` radians, then moves by `t`.
fn rot(p: V, a: f64, t: V) -> V {
    [
        p[0] * a.cos() - p[1] * a.sin() + t[0],
        p[0] * a.sin() + p[1] * a.cos() + t[1],
        p[2] + t[2],
    ]
}
fn turn(p: V, a: f64) -> V {
    rot(p, a, [0.0; 3])
}

/// 40 x 20 plate, base at z=0, holes d6 at (10,5) and (30,5), a step face at z=4, one tilted pin.
fn plate(a: f64, t: V) -> Vec<Step> {
    let (z, x, y) = ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let planes = vec![
        (
            "step".to_string(),
            plane(
                rot([5.0, 5.0, 4.0], a, t),
                turn(z, a),
                turn(x, a),
                turn(y, a),
                5.0,
                5.0,
            ),
        ),
        (
            "base".to_string(),
            plane(
                rot([20.0, 10.0, 0.0], a, t),
                turn(z, a),
                turn(x, a),
                turn(y, a),
                40.0,
                20.0,
            ),
        ),
    ];
    let tilt = [0.5f64.sqrt(), 0.0, 0.5f64.sqrt()];
    let cyls = vec![
        (
            "h1".to_string(),
            cyl(rot([10.0, 5.0, 2.0], a, t), turn(z, a), 6.0, 4.0),
        ),
        (
            "h2".to_string(),
            cyl(rot([30.0, 5.0, 2.0], a, t), turn(z, a), 6.0, 4.0),
        ),
        (
            "pin".to_string(),
            cyl(rot([20.0, 10.0, 2.0], a, t), turn(tilt, a), 3.0, 8.0),
        ),
    ];
    recipe(&planes, &cyls).steps
}

fn rounds(s: &[Step]) -> Vec<(f64, f64, f64, f64)> {
    s.iter()
        .filter_map(|s| match s {
            Step::Round {
                u,
                v,
                height,
                diameter,
                ..
            } => Some((*u, *v, *height, *diameter)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_plate_gives_base_holes_step_and_an_unplaced_pin() {
    let s = plate(0.0, [0.0; 3]);
    assert!(
        matches!(&s[0], Step::Base { name, size } if name == "base" && *size == [40.0, 20.0]),
        "{s:?}"
    );
    assert!(s.iter().any(|x| matches!(x, Step::ParallelFace { name, height } if name == "step" && (height - 4.0).abs() < 1e-9)));
    let r = rounds(&s);
    assert_eq!(r.len(), 2);
    assert!(
        (r[0].0 + 10.0).abs() < 1e-9 && (r[0].1 + 5.0).abs() < 1e-9,
        "{r:?}"
    ); // (10,5) minus base centre (20,10)
    assert!((r[1].0 - 10.0).abs() < 1e-9 && (r[1].1 + 5.0).abs() < 1e-9);
    assert!(
        r.iter()
            .all(|x| (x.2 - 2.0).abs() < 1e-9 && (x.3 - 6.0).abs() < 1e-9)
    );
    assert!(
        s.iter()
            .any(|x| matches!(x, Step::Unplaced { name, .. } if name == "pin"))
    );
}

#[test]
fn turning_and_moving_the_plate_changes_nothing() {
    let a = plate(0.0, [0.0; 3]);
    let b = plate(0.7, [100.0, -50.0, 30.0]);
    assert_eq!(a.len(), b.len());
    for (x, y) in rounds(&a).iter().zip(rounds(&b)) {
        assert!(
            (x.0 - y.0).abs() < 1e-9
                && (x.1 - y.1).abs() < 1e-9
                && (x.2 - y.2).abs() < 1e-9
                && (x.3 - y.3).abs() < 1e-9
        );
    }
}

#[test]
fn no_plane_means_nothing_is_placed_and_nothing_crashes() {
    let c = vec![("h".to_string(), cyl([0.0; 3], [0.0, 0.0, 1.0], 5.0, 5.0))];
    let s = recipe(&[], &c).steps;
    assert!(matches!(&s[..], [Step::Unplaced { .. }]));
    assert!(recipe(&[], &[]).steps.is_empty());
}

#[test]
fn the_recipe_is_plain_json() {
    let j = serde_json::to_value(recipe(
        &[(
            "p".into(),
            plane(
                [0.0; 3],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                2.0,
                1.0,
            ),
        )],
        &[],
    ))
    .unwrap();
    assert_eq!(j["steps"][0]["step"], "base");
}
