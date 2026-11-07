// SPDX-License-Identifier: MIT OR Apache-2.0
//! Automatic flat regions grown over neighbouring triangles (docs/specs/segmentation.md, part a).

use crate::fit::{Clip, PlaneFit, fit_plane};
use crate::graph::MeshGraph;
use crate::math::{Vec3, covariance, cross, dot, length, sub, symmetric_eigen3};
use crate::mesh::Mesh;
use std::collections::VecDeque;

/// Limits for growing flat regions.
#[derive(Debug, Clone, Copy)]
pub struct PlaneSegmentOptions {
    /// Largest angle between a triangle's normal and its region's plane, radians.
    pub max_angle: f64,
    /// Largest distance of a triangle's corners from its region's plane, mm.
    pub max_distance: f64,
    /// Regions with less area than this (mm²) are dropped.
    pub min_area: f64,
}

impl Default for PlaneSegmentOptions {
    fn default() -> Self {
        Self {
            max_angle: 10f64.to_radians(),
            max_distance: 0.2,
            min_area: 0.0,
        }
    }
}

/// One flat region.
#[derive(Debug, Clone)]
pub struct PlaneRegion {
    /// Its triangles, ascending.
    pub triangles: Vec<u32>,
    /// Its vertices, ascending and unique.
    pub vertices: Vec<u32>,
    /// Total triangle area, mm².
    pub area: f64,
    /// Plane fit on its vertices (no clipping, oriented by vertex normals).
    pub plane: PlaneFit,
}

/// The flat regions of a mesh, largest area first.
#[derive(Debug, Clone)]
pub struct PlaneSegmentation {
    /// Regions, largest area first.
    pub regions: Vec<PlaneRegion>,
    /// Triangles in no region (zero area, or in a region below the area limit).
    pub unassigned: usize,
}

fn corner(mesh: &Mesh, t: u32, k: usize) -> Vec3 {
    mesh.positions[mesh.triangles[t as usize][k] as usize].map(f64::from)
}

fn area(mesh: &Mesh, t: u32) -> f64 {
    let (a, b, c) = (corner(mesh, t, 0), corner(mesh, t, 1), corner(mesh, t, 2));
    0.5 * length(cross(sub(b, a), sub(c, a)))
}

/// Splits `mesh` into flat regions.
pub fn segment_planes(
    mesh: &Mesh,
    graph: &MeshGraph,
    options: PlaneSegmentOptions,
) -> PlaneSegmentation {
    let nt = mesh.triangles.len();
    let normal = |t: u32| graph.face_normals[t as usize];
    let usable = |t: u32| length(normal(t)) > 0.0;

    let neighbours = triangle_neighbours(graph, nt);
    let areas: Vec<f64> = (0..nt as u32).map(|t| area(mesh, t)).collect();

    // Seeds: flattest first (smallest largest-angle to a neighbour, i.e. largest smallest-dot).
    let flatness = |t: u32| {
        neighbours[t as usize]
            .iter()
            .filter(|&&u| usable(u))
            .map(|&u| dot(normal(t), normal(u)))
            .fold(1.0f64, f64::min)
    };
    let mut seeds: Vec<u32> = (0..nt as u32).filter(|&t| usable(t)).collect();
    let score: Vec<f64> = (0..nt as u32).map(flatness).collect();
    seeds.sort_by(|&a, &b| {
        score[b as usize]
            .total_cmp(&score[a as usize])
            .then(areas[b as usize].total_cmp(&areas[a as usize]))
            .then(a.cmp(&b))
    });

    let cos_limit = options.max_angle.cos();
    let mut owner = vec![u32::MAX; nt];
    let mut grown: Vec<Vec<u32>> = Vec::new();
    for seed in seeds {
        if owner[seed as usize] != u32::MAX {
            continue;
        }
        let id = grown.len() as u32;
        let (mut center, mut n) = (corner(mesh, seed, 0), normal(seed));
        let mut members = vec![seed];
        owner[seed as usize] = id;
        let mut last_refit = 1usize;
        let mut queue = VecDeque::from([seed]);
        while let Some(t) = queue.pop_front() {
            for &u in &neighbours[t as usize] {
                if owner[u as usize] != u32::MAX || !usable(u) {
                    continue;
                }
                let flat = dot(normal(u), n) >= cos_limit;
                let close = (0..3)
                    .all(|k| dot(sub(corner(mesh, u, k), center), n).abs() <= options.max_distance);
                if flat && close {
                    owner[u as usize] = id;
                    members.push(u);
                    queue.push_back(u);
                    if members.len() >= 2 * last_refit {
                        last_refit = members.len();
                        let mut verts: Vec<u32> = members
                            .iter()
                            .flat_map(|&m| mesh.triangles[m as usize])
                            .collect();
                        verts.sort_unstable();
                        verts.dedup();
                        if let Some(c) = covariance(
                            verts
                                .iter()
                                .map(|&v| mesh.positions[v as usize].map(f64::from)),
                        ) && let Some(e) = symmetric_eigen3(c.matrix)
                            && e.values[1] > 1e-12 * e.values[2]
                        {
                            let m = e.vectors[0];
                            // trim() afterwards keeps the region within the limit of its final plane.
                            n = if dot(m, n) < 0.0 { m.map(|x| -x) } else { m };
                            center = c.centroid;
                        }
                    }
                }
            }
        }
        trim(mesh, &mut members, &mut owner, options.max_distance);
        grown.push(members);
    }

    let mut regions = Vec::new();
    let mut unassigned = owner.iter().filter(|&&o| o == u32::MAX).count();
    for mut triangles in grown {
        let area: f64 = triangles.iter().map(|&t| areas[t as usize]).sum();
        let mut vertices: Vec<u32> = triangles
            .iter()
            .flat_map(|&t| mesh.triangles[t as usize])
            .collect();
        vertices.sort_unstable();
        vertices.dedup();
        let plane = match fit_plane(
            &mesh.positions,
            Some(&graph.vertex_normals),
            &vertices,
            Clip::None,
        ) {
            Ok(p) if area >= options.min_area => p,
            _ => {
                unassigned += triangles.len();
                continue;
            }
        };
        triangles.sort_unstable();
        regions.push(PlaneRegion {
            triangles,
            vertices,
            area,
            plane,
        });
    }
    regions.sort_by(|a, b| {
        b.area
            .total_cmp(&a.area)
            .then(a.triangles[0].cmp(&b.triangles[0]))
    });
    PlaneSegmentation {
        regions,
        unassigned,
    }
}

/// Refits the grown region and hands back every triangle with a corner past `limit` from the fit,
/// repeating until none strays, so a region always lies within the limit of its own reported plane.
/// Handed-back triangles are free for later seeds. Terminates: triangles are only removed, and one
/// triangle always fits exactly.
fn trim(mesh: &Mesh, members: &mut Vec<u32>, owner: &mut [u32], limit: f64) {
    loop {
        let mut verts: Vec<u32> = members
            .iter()
            .flat_map(|&m| mesh.triangles[m as usize])
            .collect();
        verts.sort_unstable();
        verts.dedup();
        let Some(c) = covariance(
            verts
                .iter()
                .map(|&v| mesh.positions[v as usize].map(f64::from)),
        ) else {
            return;
        };
        let Some(e) = symmetric_eigen3(c.matrix) else {
            return;
        };
        let n = e.vectors[0];
        let strays =
            |t: u32| (0..3).any(|k| dot(sub(corner(mesh, t, k), c.centroid), n).abs() > limit);
        let before = members.len();
        members.retain(|&t| {
            let keep = !strays(t);
            if !keep {
                owner[t as usize] = u32::MAX;
            }
            keep
        });
        if members.len() == before || members.len() <= 1 {
            return;
        }
    }
}

/// For each triangle, its neighbours across manifold edges.
fn triangle_neighbours(graph: &MeshGraph, nt: usize) -> Vec<Vec<u32>> {
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); nt];
    for e in &graph.edges {
        if let (Some(t1), false) = (e.t1, e.non_manifold) {
            neighbours[e.t0 as usize].push(t1);
            neighbours[t1 as usize].push(e.t0);
        }
    }
    neighbours
}

/// Limits for growing round regions.
#[derive(Debug, Clone, Copy)]
pub struct CylinderSegmentOptions {
    /// Largest angle between a triangle's normal and the cylinder's radial direction, radians.
    pub max_angle: f64,
    /// Largest distance of a triangle's corners from the cylinder surface, mm.
    pub max_distance: f64,
    /// Largest radius accepted, mm; beyond it the surface counts as flat.
    pub max_radius: f64,
    /// Smallest angular coverage kept, degrees.
    pub min_coverage_deg: f64,
}

impl Default for CylinderSegmentOptions {
    fn default() -> Self {
        Self {
            max_angle: 15f64.to_radians(),
            max_distance: 0.2,
            max_radius: 10_000.0,
            min_coverage_deg: 20.0,
        }
    }
}

/// One round region.
#[derive(Debug, Clone)]
pub struct CylinderRegion {
    /// Its triangles, ascending.
    pub triangles: Vec<u32>,
    /// Its vertices, ascending and unique.
    pub vertices: Vec<u32>,
    /// Total triangle area, mm².
    pub area: f64,
    /// Cylinder fit on its vertices.
    pub cylinder: crate::fit::CylinderFit,
}

/// The round regions of a mesh, largest area first.
#[derive(Debug, Clone)]
pub struct CylinderSegmentation {
    /// Regions, largest area first.
    pub regions: Vec<CylinderRegion>,
    /// Triangles in no round region.
    pub unassigned: usize,
}

struct Axis {
    point: Vec3,
    dir: Vec3,
    radius: f64,
}

impl Axis {
    fn of(c: &crate::fit::CylinderFit) -> Axis {
        Axis {
            point: c.axis_point,
            dir: c.axis,
            radius: c.radius,
        }
    }
    /// Offset of `p` from the axis line, perpendicular to it.
    fn radial(&self, p: Vec3) -> Vec3 {
        let d = sub(p, self.point);
        let along = dot(d, self.dir);
        [
            d[0] - along * self.dir[0],
            d[1] - along * self.dir[1],
            d[2] - along * self.dir[2],
        ]
    }
    fn surface_distance(&self, p: Vec3) -> f64 {
        (length(self.radial(p)) - self.radius).abs()
    }
}

fn vertices_of(mesh: &Mesh, triangles: &[u32]) -> Vec<u32> {
    let mut v: Vec<u32> = triangles
        .iter()
        .flat_map(|&t| mesh.triangles[t as usize])
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

fn fit_round(
    mesh: &Mesh,
    graph: &MeshGraph,
    triangles: &[u32],
    max_radius: f64,
) -> Option<crate::fit::CylinderFit> {
    let verts = vertices_of(mesh, triangles);
    let fit = crate::fit::fit_cylinder(
        &mesh.positions,
        Some(&graph.vertex_normals),
        &verts,
        Default::default(),
    )
    .ok()?;
    (fit.radius.is_finite() && fit.radius > 0.0 && fit.radius <= max_radius).then_some(fit)
}

/// Finds round regions (holes, bosses, fillets) in `mesh`.
pub fn segment_cylinders(
    mesh: &Mesh,
    graph: &MeshGraph,
    options: CylinderSegmentOptions,
) -> CylinderSegmentation {
    let nt = mesh.triangles.len();
    let neighbours = triangle_neighbours(graph, nt);
    let normal = |t: u32| graph.face_normals[t as usize];
    let usable = |t: u32| length(normal(t)) > 0.0;
    let areas: Vec<f64> = (0..nt as u32).map(|t| area(mesh, t)).collect();
    let (lo, hi) = (45f64.to_radians().cos(), 0.5f64.to_radians().cos());
    let curving = |t: u32| {
        let worst = neighbours[t as usize]
            .iter()
            .filter(|&&u| usable(u))
            .map(|&u| dot(normal(t), normal(u)))
            .fold(1.0f64, f64::min);
        worst >= lo && worst <= hi
    };
    let centroid = |t: u32| {
        let (a, b, c) = (corner(mesh, t, 0), corner(mesh, t, 1), corner(mesh, t, 2));
        [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ]
    };
    let sin_limit = options.max_angle.sin();
    let fits = |t: u32, axis: &Axis| {
        let radial = crate::math::normalize(axis.radial(centroid(t)));
        let aligned = radial.is_some_and(|r| {
            let n = normal(t);
            // |sin| of the angle between normal and radial direction, either sign.
            length(cross(n, r)) <= sin_limit
        });
        aligned
            && surface_samples(mesh, t)
                .iter()
                .all(|&p| axis.surface_distance(p) <= options.max_distance)
    };

    let mut owner = vec![u32::MAX; nt];
    let mut regions = Vec::new();
    for seed in 0..nt as u32 {
        if owner[seed as usize] != u32::MAX || !usable(seed) || !curving(seed) {
            continue;
        }
        // Seed patch: the seed and every free triangle within two edge-steps.
        let mut patch = vec![seed];
        for _ in 0..2 {
            let ring: Vec<u32> = patch
                .iter()
                .flat_map(|&t| neighbours[t as usize].iter().copied())
                .collect();
            for u in ring {
                if owner[u as usize] == u32::MAX && usable(u) && !patch.contains(&u) {
                    patch.push(u);
                }
            }
        }
        let Some(first) = fit_round(mesh, graph, &patch, options.max_radius) else {
            continue;
        };
        let mut axis = Axis::of(&first);
        if !patch.iter().all(|&t| fits(t, &axis)) {
            continue;
        }
        let id = regions.len() as u32 + 1;
        for &t in &patch {
            owner[t as usize] = id;
        }
        let mut members = patch.clone();
        let mut last_refit = members.len();
        let mut queue: VecDeque<u32> = patch.into_iter().collect();
        while let Some(t) = queue.pop_front() {
            for &u in &neighbours[t as usize] {
                if owner[u as usize] != u32::MAX || !usable(u) || !fits(u, &axis) {
                    continue;
                }
                owner[u as usize] = id;
                members.push(u);
                queue.push_back(u);
                if members.len() >= 2 * last_refit {
                    last_refit = members.len();
                    if let Some(f) = fit_round(mesh, graph, &members, options.max_radius) {
                        axis = Axis::of(&f);
                    }
                }
            }
        }
        // Trim until every member lies within the limit of the region's own final fit.
        let fit = loop {
            let Some(f) = fit_round(mesh, graph, &members, options.max_radius) else {
                break None;
            };
            let a = Axis::of(&f);
            let before = members.len();
            members.retain(|&t| {
                let keep = surface_samples(mesh, t)
                    .iter()
                    .all(|&p| a.surface_distance(p) <= options.max_distance);
                if !keep {
                    owner[t as usize] = u32::MAX;
                }
                keep
            });
            if members.len() == before {
                break Some(f);
            }
            if members.len() < 3 {
                break None;
            }
        };
        match fit {
            Some(cylinder) if cylinder.coverage_deg >= options.min_coverage_deg => {
                members.sort_unstable();
                let area = members.iter().map(|&t| areas[t as usize]).sum();
                let vertices = vertices_of(mesh, &members);
                regions.push(CylinderRegion {
                    triangles: members,
                    vertices,
                    area,
                    cylinder,
                });
            }
            _ => {
                for &t in &members {
                    owner[t as usize] = u32::MAX;
                }
            }
        }
    }
    regions.sort_by(|a, b| {
        b.area
            .total_cmp(&a.area)
            .then(a.triangles[0].cmp(&b.triangles[0]))
    });
    let assigned: usize = regions.iter().map(|r| r.triangles.len()).sum();
    CylinderSegmentation {
        regions,
        unassigned: nt - assigned,
    }
}

/// Points on a triangle's own surface: its corners, edge midpoints and centroid. Checking only corners
/// lets a large flat triangle whose corners sit on a big cylinder pass, though its middle sags off it.
fn surface_samples(mesh: &Mesh, t: u32) -> [Vec3; 7] {
    let (a, b, c) = (corner(mesh, t, 0), corner(mesh, t, 1), corner(mesh, t, 2));
    let mid = |p: Vec3, q: Vec3| {
        [
            (p[0] + q[0]) / 2.0,
            (p[1] + q[1]) / 2.0,
            (p[2] + q[2]) / 2.0,
        ]
    };
    let g = [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ];
    [a, b, c, mid(a, b), mid(b, c), mid(c, a), g]
}
