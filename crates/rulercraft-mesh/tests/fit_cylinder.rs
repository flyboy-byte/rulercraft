// SPDX-License-Identifier: MIT OR Apache-2.0
//! Known-answer cylinder surfaces in millimetres.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{
    fit::{
        Clip,
        cylinder::{CylinderLimits, CylinderOptions, fit_cylinder},
    },
    math::{add, cross, dot, normalize, scale},
};
fn tube(axis: [f64; 3], arc: f64) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>) {
    let a = normalize(axis).unwrap();
    let u = normalize(cross(a, [0., 1., 0.])).unwrap();
    let v = cross(a, u);
    let mut p = Vec::new();
    let mut n = Vec::new();
    for z in [-20., 0., 20.] {
        for i in 0..64 {
            let theta = if arc == 360. {
                i as f64 * std::f64::consts::TAU / 64.
            } else {
                (-170. + arc * i as f64 / 63.).to_radians()
            };
            let radial = add(scale(u, theta.cos()), scale(v, theta.sin()));
            p.push(add([3., -7., 11.], add(scale(a, z), scale(radial, 10.))).map(|v| v as f32));
            n.push(radial.map(|v| v as f32));
        }
    }
    let ids = (0..p.len() as u32).collect();
    (p, n, ids)
}
#[test]
fn exact_tube_and_metadata() {
    let (p, n, ids) = tube([0., 0., 1.], 360.);
    let r = fit_cylinder(
        &p,
        Some(&n),
        &ids,
        CylinderOptions {
            assumed_diameter: Some(21.),
            extend_before: 2.,
            extend_after: 3.,
            ..Default::default()
        },
    )
    .unwrap();
    assert!((r.radius - 10.).abs() < 1e-5);
    assert!((r.diameter - 20.).abs() < 2e-5); // f32 input quantization, fixed by spec
    assert!(dot(r.axis, [0., 0., 1.]) > 1. - 1e-6);
    assert!((r.length - 40.).abs() < 1e-5);
    for (x, y) in r.axis_point.into_iter().zip([3., -7., 11.]) {
        assert!((x - y).abs() < 1e-5);
    }
    assert!((r.coverage_deg - (360. - 360. / 64.)).abs() < 1e-4);
    assert_eq!(
        (r.assumed_diameter, r.extend_before, r.extend_after),
        (Some(21.), 2., 3.)
    );
    assert_eq!((r.used, r.offered), (192, 192));
    assert!(r.sigma < 1e-5);
}
#[test]
fn partial_arc_tilt_and_missing_normals() {
    for arc in [120., 360.] {
        let (p, n, ids) = tube([1., 2., 3.], arc);
        for normals in [Some(n.as_slice()), None] {
            let r = fit_cylinder(&p, normals, &ids, CylinderOptions::default()).unwrap();
            assert!((r.radius - 10.).abs() < 1e-5, "{r:?}");
            assert!(dot(r.axis, normalize([1., 2., 3.]).unwrap()).abs() > 1. - 1e-4);
            if arc == 120. {
                assert!((r.coverage_deg - 120.).abs() < 1e-4);
            }
        }
    }
}
#[test]
fn clipping_recovers_radius_from_small_radial_spikes() {
    let (mut p, n, ids) = tube([0., 0., 1.], 360.);
    for i in [5, 70, 140] {
        for k in 0..3 {
            p[i][k] += 0.2 * n[i][k];
        }
    }
    let r = fit_cylinder(
        &p,
        Some(&n),
        &ids,
        CylinderOptions {
            clip: Clip::Sigma(3),
            ..Default::default()
        },
    )
    .unwrap();
    assert!((r.radius - 10.).abs() < 1e-5);
    assert_eq!(r.used, 189);
    assert!(r.rounds > 1);
}
#[test]
fn flat_bad_inputs_and_limits_refuse() {
    let (p, n, ids) = tube([0., 0., 1.], 360.);
    let flat: Vec<_> = p.iter().map(|v| [v[0], v[1], 0.]).collect();
    assert!(fit_cylinder(&flat, Some(&n), &ids, CylinderOptions::default()).is_err());
    for options in [
        CylinderOptions {
            clip: Clip::Sigma(0),
            ..Default::default()
        },
        CylinderOptions {
            assumed_diameter: Some(-1.),
            ..Default::default()
        },
        CylinderOptions {
            extend_before: -1.,
            ..Default::default()
        },
        CylinderOptions {
            limits: CylinderLimits {
                max_points: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        CylinderOptions {
            limits: CylinderLimits {
                max_iterations: 0,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(fit_cylinder(&p, Some(&n), &ids, options).is_err());
    }
    assert!(fit_cylinder(&p, Some(&n), &[0, 1], CylinderOptions::default()).is_err());
    assert!(
        fit_cylinder(
            &p,
            Some(&n),
            &[0, 1, 2, 3, 4, 999],
            CylinderOptions::default()
        )
        .is_err()
    );
    assert!(fit_cylinder(&p, Some(&n[..1]), &ids, CylinderOptions::default()).is_err());
    let mut bad = p.clone();
    bad[0][0] = f32::NAN;
    assert!(fit_cylinder(&bad, None, &ids, CylinderOptions::default()).is_err());
}

#[test]
fn refinement_corrects_a_tilted_normal_start() {
    let (p, mut n, ids) = tube([0., 0., 1.], 120.);
    // Deliberately skew normal directions so their perpendicular axis is tilted by atan(0.1).
    for normal in &mut n {
        normal[2] = 0.1 * normal[0];
    }
    let fit = fit_cylinder(&p, Some(&n), &ids, CylinderOptions::default()).unwrap();
    assert!((fit.radius - 10.).abs() < 1e-5);
    assert!((fit.axis[2] - 1.).abs() < 1e-6);
    assert!(fit.sigma < 1e-5);
}

#[test]
fn symmetric_origin_tube_does_not_refuse_a_stationary_fit() {
    let mut data = String::from(
        "ply\nformat ascii 1.0\nelement vertex 96\nproperty float x\nproperty float y\nproperty float z\nelement face 128\nproperty list uchar int vertex_indices\nend_header\n",
    );
    for z in [0., 20., 40.] {
        for i in 0..32 {
            let a = i as f64 * std::f64::consts::TAU / 32.;
            data += &format!("{} {} {}\n", 10. * a.cos(), 10. * a.sin(), z);
        }
    }
    for ring in 0..2 {
        for i in 0..32 {
            let a = ring * 32 + i;
            let b = ring * 32 + (i + 1) % 32;
            data += &format!("3 {a} {b} {}\n3 {a} {} {}\n", b + 32, b + 32, a + 32);
        }
    }
    let mesh = rulercraft_mesh::read_mesh(
        "tube.ply",
        data.as_bytes(),
        rulercraft_mesh::Unit::Mm,
        rulercraft_mesh::ImportLimits::default(),
    )
    .unwrap();
    let n = rulercraft_mesh::graph::MeshGraph::build(&mesh).vertex_normals;
    let ids: Vec<_> = (0..mesh.positions.len() as u32).collect();
    let fit = fit_cylinder(&mesh.positions, Some(&n), &ids, CylinderOptions::default()).unwrap();
    assert!((fit.radius - 10.).abs() < 1e-5); // unchanged spec tolerance for f32 positions
    assert!((fit.axis[2] - 1.).abs() < 1e-6);
    assert!((fit.length - 40.).abs() < 1e-5);
    let mut tilted = n;
    for normal in &mut tilted {
        normal[2] = 0.1 * normal[0];
    }
    assert!(
        fit_cylinder(
            &mesh.positions,
            Some(&tilted),
            &ids,
            CylinderOptions {
                limits: CylinderLimits {
                    max_iterations: 1,
                    ..Default::default()
                },
                ..Default::default()
            }
        )
        .is_err()
    );
}
