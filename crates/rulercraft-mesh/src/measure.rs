// SPDX-License-Identifier: MIT OR Apache-2.0
//! Constant-work relations between named geometric elements; lengths in mm and angles in radians.
use crate::math::{Vec3, cross, dot, length, normalize, scale, sub};
use std::fmt;
/// Geometry needed for a relation; cylinder sizes do not affect centre-line measurements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Geometry {
    /// A point, mm.
    Point(Vec3),
    /// A sphere measured by its centre.
    Sphere {
        /// Centre, mm.
        center: Vec3,
    },
    /// Infinite plane.
    Plane {
        /// Point on plane, mm.
        center: Vec3,
        /// Nonzero normal, normalized during measurement.
        normal: Vec3,
    },
    /// Infinite cylinder axis.
    Cylinder {
        /// Point on axis, mm.
        axis_point: Vec3,
        /// Nonzero direction, normalized during measurement.
        axis: Vec3,
    },
}
/// A named element used by a measurement.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Caller-selected identity, returned unchanged.
    pub id: String,
    /// Measured geometry.
    pub geometry: Geometry,
}
/// Typed measurement command.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureRequest {
    /// First element; sets the origin for directional conventions.
    pub first: Element,
    /// Second element.
    pub second: Element,
}
/// The measured pair category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    /// Point or sphere-centre pair.
    PointPoint,
    /// Point or centre to plane.
    PointPlane,
    /// Plane pair.
    PlanePlane,
    /// Axis to plane.
    AxisPlane,
    /// Axis pair.
    AxisAxis,
}
/// Plain measurement data with explicit conventions.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// First element identity.
    pub first_id: String,
    /// Second element identity.
    pub second_id: String,
    /// Pair category.
    pub kind: RelationKind,
    /// Unsigned acute angle, radians; None for point relations.
    pub angle: Option<f64>,
    /// Distance, mm; signed only for point-plane. None when only an angle is defined.
    pub distance: Option<f64>,
    /// Second minus first coordinate components, only for point/centre pairs.
    pub delta: Option<Vec3>,
    /// Stable name explaining how the distance/angle was defined.
    pub convention: &'static str,
}
/// Invalid geometry or an unsupported element pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureError(pub &'static str);
impl fmt::Display for MeasureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for MeasureError {}
fn validated(g: Geometry) -> Result<Geometry, MeasureError> {
    let point = match g {
        Geometry::Point(p) => p,
        Geometry::Sphere { center } | Geometry::Plane { center, .. } => center,
        Geometry::Cylinder { axis_point, .. } => axis_point,
    };
    if point.iter().any(|v| !v.is_finite()) {
        return Err(MeasureError("element position must be finite"));
    }
    let direction =
        |v: Vec3| normalize(v).ok_or(MeasureError("element direction must be finite and nonzero"));
    match g {
        Geometry::Plane { center, normal } => Ok(Geometry::Plane {
            center,
            normal: direction(normal)?,
        }),
        Geometry::Cylinder { axis_point, axis } => Ok(Geometry::Cylinder {
            axis_point,
            axis: direction(axis)?,
        }),
        Geometry::Sphere { center } => Ok(Geometry::Point(center)),
        other => Ok(other),
    }
}
fn angle(a: Vec3, b: Vec3) -> f64 {
    length(cross(a, b)).atan2(dot(a, b).abs())
}
/// Runs a relation command, refusing non-finite results and unsupported pairs.
pub fn measure(request: &MeasureRequest) -> Result<Measurement, MeasureError> {
    let a = validated(request.first.geometry)?;
    let b = validated(request.second.geometry)?;
    let mut out = Measurement {
        first_id: request.first.id.clone(),
        second_id: request.second.id.clone(),
        kind: RelationKind::PointPoint,
        angle: None,
        distance: None,
        delta: None,
        convention: "second_minus_first",
    };
    match (a, b) {
        (Geometry::Point(a), Geometry::Point(b)) => {
            let d = sub(b, a);
            out.delta = Some(d);
            out.distance = Some(length(d));
        }
        (Geometry::Point(p), Geometry::Plane { center, normal })
        | (Geometry::Plane { center, normal }, Geometry::Point(p)) => {
            out.kind = RelationKind::PointPlane;
            out.distance = Some(dot(sub(p, center), normal));
            out.convention = "signed_along_plane_normal";
        }
        (
            Geometry::Plane {
                center: a,
                normal: na,
            },
            Geometry::Plane {
                center: b,
                normal: nb,
            },
        ) => {
            out.kind = RelationKind::PlanePlane;
            let angle = angle(na, nb);
            out.angle = Some(angle);
            if angle < 1_f64.to_radians() {
                out.distance = Some(dot(sub(a, b), nb).abs());
                out.convention = "distance_at_first_plane_center";
            } else {
                out.convention = "angle_only_intersecting_planes";
            }
        }
        (Geometry::Cylinder { axis, .. }, Geometry::Plane { normal, .. })
        | (Geometry::Plane { normal, .. }, Geometry::Cylinder { axis, .. }) => {
            out.kind = RelationKind::AxisPlane;
            out.angle = Some(dot(axis, normal).abs().atan2(length(cross(axis, normal))));
            out.convention = "acute_axis_to_plane";
        }
        (
            Geometry::Cylinder {
                axis_point: a,
                axis: da,
            },
            Geometry::Cylinder {
                axis_point: b,
                axis: db,
            },
        ) => {
            out.kind = RelationKind::AxisAxis;
            out.angle = Some(angle(da, db));
            let delta = sub(b, a);
            let n = cross(da, db);
            let sine = length(n);
            if sine <= 1e-12 {
                out.distance = Some(length(sub(delta, scale(da, dot(delta, da)))));
                out.convention = "parallel_at_first_axis";
            } else {
                out.distance = Some(dot(delta, scale(n, 1. / sine)).abs());
                out.convention = "closest_infinite_lines";
            }
        }
        _ => {
            return Err(MeasureError(
                "unsupported element pair: use points/centres, point-plane, plane-plane, axis-plane or axis-axis",
            ));
        }
    }
    if out
        .angle
        .into_iter()
        .chain(out.distance)
        .chain(out.delta.into_iter().flatten())
        .any(|v| !v.is_finite())
    {
        return Err(MeasureError(
            "measurement overflow: element coordinates are too far apart",
        ));
    }
    Ok(out)
}
