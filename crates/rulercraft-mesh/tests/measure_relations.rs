// SPDX-License-Identifier: MIT OR Apache-2.0
//! Closed-form distances and angles, including parallel and reversed directions.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::measure::{Element, Geometry, MeasureRequest, Measurement, measure};
fn run(a: Geometry, b: Geometry) -> Measurement {
    measure(&MeasureRequest {
        first: Element {
            id: "first".into(),
            geometry: a,
        },
        second: Element {
            id: "second".into(),
            geometry: b,
        },
    })
    .unwrap()
}
fn plane(center: [f64; 3], normal: [f64; 3]) -> Geometry {
    Geometry::Plane { center, normal }
}
fn axis(origin: [f64; 3], direction: [f64; 3]) -> Geometry {
    Geometry::Cylinder {
        axis_point: origin,
        axis: direction,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
} // f64 vector arithmetic roundoff
#[test]
fn point_centres_and_signed_plane_distance() {
    let m = run(
        Geometry::Point([1., 2., 3.]),
        Geometry::Sphere {
            center: [4., 6., 3.],
        },
    );
    assert_eq!(m.delta, Some([3., 4., 0.]));
    assert_eq!(m.distance, Some(5.));
    assert_eq!(m.first_id, "first");
    assert_eq!(m.second_id, "second");
    for reverse in [false, true] {
        let a = Geometry::Point([0., 0., -3.]);
        let b = plane([0., 0., 2.], [0., 0., 2.]);
        let m = if reverse { run(b, a) } else { run(a, b) };
        assert_eq!(m.distance, Some(-5.));
        assert_eq!(m.convention, "signed_along_plane_normal");
    }
}
#[test]
fn parallel_tilted_and_opposite_planes() {
    let m = run(
        plane([0.; 3], [0., 0., 1.]),
        plane([0., 0., 7.], [0., 0., -2.]),
    );
    assert_eq!(m.angle, Some(0.));
    assert_eq!(m.distance, Some(7.));
    let m = run(plane([0.; 3], [0., 0., 1.]), plane([0.; 3], [1., 0., 0.]));
    close(m.angle.unwrap(), std::f64::consts::FRAC_PI_2);
    assert_eq!(m.distance, None);
    let a = 0.5_f64.to_radians();
    let m = run(
        plane([0.; 3], [0., 0., 1.]),
        plane([0., 0., 5.], [a.sin(), 0., a.cos()]),
    );
    close(m.angle.unwrap(), a);
    close(m.distance.unwrap(), 5. * a.cos());
    assert_eq!(m.convention, "distance_at_first_plane_center");
}
#[test]
fn axis_angles_skew_and_parallel_spacing() {
    let m = run(
        axis([0.; 3], [1., 0., 0.]),
        axis([0., 0., 4.], [0., 2., 0.]),
    );
    close(m.angle.unwrap(), std::f64::consts::FRAC_PI_2);
    assert_eq!(m.distance, Some(4.));
    assert_eq!(m.convention, "closest_infinite_lines");
    let m = run(
        axis([0.; 3], [0., 0., 1.]),
        axis([3., 4., 99.], [0., 0., -2.]),
    );
    assert_eq!(m.angle, Some(0.));
    assert_eq!(m.distance, Some(5.));
    assert_eq!(m.convention, "parallel_at_first_axis");
    let m = run(
        axis([0.; 3], [1., 0., 0.]),
        axis([0., 2., 0.], [1., 1e-14, 0.]),
    );
    close(m.distance.unwrap(), 2.);
    assert_eq!(m.convention, "parallel_at_first_axis");
    for (d, a) in [
        ([1., 0., 0.], 0.),
        ([0., 0., 1.], std::f64::consts::FRAC_PI_2),
    ] {
        let m = run(plane([0.; 3], [0., 0., 1.]), axis([0.; 3], d));
        close(m.angle.unwrap(), a);
        assert_eq!(m.distance, None);
    }
}
#[test]
fn zero_nonfinite_overflow_and_unsupported_pairs_refuse() {
    for (a, b) in [
        (plane([0.; 3], [0.; 3]), Geometry::Point([0.; 3])),
        (
            Geometry::Point([f64::NAN, 0., 0.]),
            Geometry::Point([0.; 3]),
        ),
        (
            Geometry::Point([f64::MAX, 0., 0.]),
            Geometry::Point([-f64::MAX, 0., 0.]),
        ),
        (axis([0.; 3], [0., 0., 1.]), Geometry::Point([0.; 3])),
    ] {
        assert!(
            measure(&MeasureRequest {
                first: Element {
                    id: "a".into(),
                    geometry: a
                },
                second: Element {
                    id: "b".into(),
                    geometry: b
                }
            })
            .is_err()
        );
    }
}
