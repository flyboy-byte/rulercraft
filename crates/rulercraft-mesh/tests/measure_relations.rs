// SPDX-License-Identifier: MIT OR Apache-2.0
//! measure.relations checks: element pairs whose distances and angles are known exactly.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::measure::{Element, relate};

fn close(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|a| (a - b).abs() < 1e-9)
}

fn plane(center: [f64; 3], normal: [f64; 3]) -> Element {
    Element::Plane {
        center,
        normal: rulercraft_mesh::math::normalize(normal).unwrap(),
    }
}
fn cyl(point: [f64; 3], axis: [f64; 3], radius: f64) -> Element {
    Element::Cylinder {
        point,
        axis: rulercraft_mesh::math::normalize(axis).unwrap(),
        radius,
    }
}

#[test]
fn opposite_faces_of_a_block_are_parallel_at_their_thickness() {
    // Outward normals point opposite ways; the faces are still parallel, 12.5 mm apart.
    let top = plane([3.0, 4.0, 12.5], [0.0, 0.0, 1.0]);
    let bottom = plane([9.0, -2.0, 0.0], [0.0, 0.0, -1.0]);
    let r = relate(&top, &bottom);
    assert!(close(r.angle_deg, 0.0) && close(r.distance, 12.5), "{r:?}");
    let side = plane([0.0; 3], [1.0, 0.0, 0.0]);
    let r = relate(&top, &side);
    assert!(close(r.angle_deg, 90.0) && r.distance.is_none(), "{r:?}");
    let tilted = plane([0.0; 3], [0.0, 1.0, 1.0]);
    assert!(close(relate(&top, &tilted).angle_deg, 45.0));
}

#[test]
fn centres_to_planes_are_signed_and_centres_to_each_other_have_components() {
    let floor = plane([0.0; 3], [0.0, 0.0, 1.0]);
    let ball = Element::Sphere {
        center: [1.0, 2.0, 7.0],
        radius: 3.0,
    };
    assert!(close(relate(&floor, &ball).distance, 7.0));
    assert!(close(relate(&ball, &floor).distance, 7.0), "either order");
    assert!(
        close(
            relate(&floor, &Element::Point([0.0, 0.0, -2.0])).distance,
            -2.0
        ),
        "below the face is negative"
    );
    let r = relate(&ball, &Element::Point([4.0, 6.0, 7.0]));
    assert!(close(r.distance, 5.0));
    assert_eq!(r.components, Some([3.0, 4.0, 0.0]));
}

#[test]
fn hole_spacing_skew_axes_and_axes_against_planes() {
    // Two parallel holes 25 mm apart (axis points offset along the axis too: spacing ignores that).
    let a = cyl([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 2.5);
    let b = cyl([15.0, 20.0, 40.0], [0.0, 0.0, -1.0], 2.5);
    let r = relate(&a, &b);
    assert!(close(r.angle_deg, 0.0) && close(r.distance, 25.0), "{r:?}");
    // Skew: an x-axis line and a y-direction line 7 mm above it.
    let r = relate(
        &cyl([0.0; 3], [1.0, 0.0, 0.0], 1.0),
        &cyl([3.0, -2.0, 7.0], [0.0, 1.0, 0.0], 1.0),
    );
    assert!(close(r.angle_deg, 90.0) && close(r.distance, 7.0), "{r:?}");
    // A hole through a plate is perpendicular to it; a pin lying on a face is parallel, at its height.
    let face = plane([0.0; 3], [0.0, 0.0, 1.0]);
    assert!(close(relate(&face, &a).angle_deg, 90.0));
    let lying = cyl([0.0, 0.0, 4.0], [1.0, 1.0, 0.0], 4.0);
    let r = relate(&lying, &face);
    assert!(close(r.angle_deg, 0.0) && close(r.distance, 4.0), "{r:?}");
    // A ball centre to an axis line.
    let r = relate(
        &a,
        &Element::Sphere {
            center: [3.0, 4.0, 100.0],
            radius: 1.0,
        },
    );
    assert!(close(r.distance, 5.0));
}

#[test]
fn fitted_holes_measure_their_designed_spacing() {
    // Two tubes of radius 2.5 with axes 25 mm apart, fitted, then measured: the designed spacing.
    let tube = |cx: f64| -> Vec<[f32; 3]> {
        let mut p = vec![];
        for ring in 0..6 {
            for i in 0..24 {
                let t = f64::from(i) * std::f64::consts::TAU / 24.0;
                p.push([
                    (cx + 2.5 * t.cos()) as f32,
                    (2.5 * t.sin()) as f32,
                    ring as f32 * 2.0,
                ]);
            }
        }
        p
    };
    let mut positions = tube(0.0);
    positions.extend(tube(25.0));
    let normals: Vec<[f32; 3]> = positions
        .iter()
        .map(|p| {
            let cx = if p[0] > 12.5 { 25.0 } else { 0.0 };
            let (x, y) = (f64::from(p[0]) - cx, f64::from(p[1]));
            let l = x.hypot(y);
            [(x / l) as f32, (y / l) as f32, 0.0]
        })
        .collect();
    let first: Vec<u32> = (0..144).collect();
    let second: Vec<u32> = (144..288).collect();
    let opts = rulercraft_mesh::fit::CylinderOptions::default();
    let a = rulercraft_mesh::fit::fit_cylinder(&positions, Some(&normals), &first, opts).unwrap();
    let b = rulercraft_mesh::fit::fit_cylinder(&positions, Some(&normals), &second, opts).unwrap();
    let r = relate(&Element::from(&a), &Element::from(&b));
    assert!(r.distance.is_some_and(|d| (d - 25.0).abs() < 1e-4), "{r:?}");
    assert!(r.angle_deg.is_some_and(|d| d < 1e-3));
}
