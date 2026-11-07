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

    // Triangle neighbours across manifold edges.
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); nt];
    for e in &graph.edges {
        if let (Some(t1), false) = (e.t1, e.non_manifold) {
            neighbours[e.t0 as usize].push(t1);
            neighbours[t1 as usize].push(e.t0);
        }
    }
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
