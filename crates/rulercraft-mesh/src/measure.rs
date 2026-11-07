// SPDX-License-Identifier: MIT OR Apache-2.0
//! Distances and angles between fitted elements (docs/specs/measure.md). Lengths in mm, angles in
//! degrees, every result saying which convention it used.

pub mod thickness;

use crate::fit::{CylinderFit, PlaneFit, SphereFit};
use crate::math::{Vec3, angle_between, cross, dot, length, normalize, sub};

/// Below this angle (degrees), plane-related distances use the near-parallel convention.
pub const PARALLEL_DEG: f64 = 1.0;

/// A fitted element reduced to what relations need.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Element {
    /// A point.
    Point(Vec3),
    /// A plane through `center` with unit `normal`.
    Plane {
        /// A point on the plane.
        center: Vec3,
        /// Unit normal.
        normal: Vec3,
    },
    /// A sphere (relations use its centre).
    Sphere {
        /// Centre.
        center: Vec3,
        /// Radius.
        radius: f64,
    },
    /// A cylinder (relations use its axis line).
    Cylinder {
        /// A point on the axis.
        point: Vec3,
        /// Unit axis direction.
        axis: Vec3,
        /// Radius.
        radius: f64,
    },
}

impl From<&PlaneFit> for Element {
    fn from(f: &PlaneFit) -> Self {
        Element::Plane {
            center: f.center,
            normal: f.normal,
        }
    }
}
impl From<&SphereFit> for Element {
    fn from(f: &SphereFit) -> Self {
        Element::Sphere {
            center: f.center,
            radius: f.radius,
        }
    }
}
impl From<&CylinderFit> for Element {
    fn from(f: &CylinderFit) -> Self {
        Element::Cylinder {
            point: f.axis_point,
            axis: f.axis,
            radius: f.radius,
        }
    }
}

/// A measured relation. Fields that do not apply to the pair are `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    /// Distance, mm (signed where `convention` says so).
    pub distance: Option<f64>,
    /// x, y, z components of a centre-to-centre distance, mm.
    pub components: Option<Vec3>,
    /// Angle, degrees.
    pub angle_deg: Option<f64>,
    /// How the numbers were taken, in words.
    pub convention: &'static str,
}

impl Relation {
    fn new(convention: &'static str) -> Self {
        Relation {
            distance: None,
            components: None,
            angle_deg: None,
            convention,
        }
    }
}

/// Acute angle between two lines or normals, degrees in [0, 90].
fn acute(a: Vec3, b: Vec3) -> f64 {
    let d = angle_between(a, b).unwrap_or(0.0).to_degrees();
    if d > 90.0 { 180.0 - d } else { d }
}

fn centre(e: &Element) -> Option<Vec3> {
    match *e {
        Element::Point(p) => Some(p),
        Element::Sphere { center, .. } => Some(center),
        _ => None,
    }
}

/// Distance from `p` to the line through `q` along unit `u`.
fn point_line(p: Vec3, q: Vec3, u: Vec3) -> f64 {
    length(cross(sub(p, q), u))
}

/// The relation between two elements, in either order.
pub fn relate(a: &Element, b: &Element) -> Relation {
    use Element::*;
    match (a, b) {
        (
            Plane {
                center: c1,
                normal: n1,
            },
            Plane {
                center: c2,
                normal: n2,
            },
        ) => {
            let angle = acute(*n1, *n2);
            let mut r = Relation::new(
                "acute angle between the planes; distance only when parallel, from the first plane's centre to the second plane",
            );
            r.angle_deg = Some(angle);
            if angle < PARALLEL_DEG {
                r.distance = Some(dot(sub(*c1, *c2), *n2).abs());
            }
            r
        }
        (Plane { center, normal }, other) | (other, Plane { center, normal })
            if centre(other).is_some() =>
        {
            let p = centre(other).unwrap_or(*center);
            let mut r = Relation::new(
                "signed distance from the plane to the point or centre, positive on the side the normal faces",
            );
            r.distance = Some(dot(sub(p, *center), *normal));
            r
        }
        (Plane { center, normal }, Cylinder { point, axis, .. })
        | (Cylinder { point, axis, .. }, Plane { center, normal }) => {
            let between = acute(*axis, *normal);
            let mut r = Relation::new(
                "angle between the axis and the plane (0 = axis lies along the plane, 90 = axis perpendicular to it); distance only when the axis is parallel to the plane",
            );
            r.angle_deg = Some(90.0 - between);
            if 90.0 - between < PARALLEL_DEG {
                r.distance = Some(dot(sub(*point, *center), *normal).abs());
            }
            r
        }
        (
            Cylinder {
                point: p1,
                axis: u1,
                ..
            },
            Cylinder {
                point: p2,
                axis: u2,
                ..
            },
        ) => {
            let angle = acute(*u1, *u2);
            let mut r = Relation::new(
                "acute angle between the axes and closest infinite-axis distance; centre-line spacing when parallel within 1e-12 cross-product magnitude",
            );
            r.angle_deg = Some(angle);
            let n = cross(*u1, *u2);
            r.distance = Some(match normalize(n) {
                Some(n) if length(cross(*u1, *u2)) > 1e-12 => dot(sub(*p2, *p1), n).abs(),
                _ => point_line(*p2, *p1, *u1),
            });
            r
        }
        (Cylinder { point, axis, .. }, other) | (other, Cylinder { point, axis, .. }) => {
            let p = centre(other).unwrap_or(*point);
            let mut r =
                Relation::new("distance from the point or centre to the cylinder's axis line");
            r.distance = Some(point_line(p, *point, *axis));
            r
        }
        _ => {
            let (p, q) = (centre(a).unwrap_or([0.0; 3]), centre(b).unwrap_or([0.0; 3]));
            let d = sub(q, p);
            let mut r = Relation::new(
                "distance between the points or centres, with x, y, z components from the first to the second",
            );
            r.distance = Some(length(d));
            r.components = Some(d);
            r
        }
    }
}
