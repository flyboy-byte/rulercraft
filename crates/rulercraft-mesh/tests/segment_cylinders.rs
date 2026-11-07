// SPDX-License-Identifier: MIT OR Apache-2.0
//! segment.cylinders checks: shapes whose round surfaces are known exactly.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::math::{cross, dot, length, sub};
use rulercraft_mesh::segment::{CylinderSegmentOptions, CylinderSegmentation, segment_cylinders};

/// Extrudes a closed profile in the x-z plane along y from 0 to `depth`, open at both ends, with
/// `rows` rows of quads. Quads are wound consistently.
fn extrude(profile: &[[f64; 2]], depth: f64, rows: u32) -> Mesh {
    let n = profile.len() as u32;
    let mut positions = vec![];
    for r in 0..=rows {
        let y = depth * f64::from(r) / f64::from(rows);
        positions.extend(profile.iter().map(|p| [p[0] as f32, y as f32, p[1] as f32]));
    }
    let mut triangles = vec![];
    for r in 0..rows {
        for i in 0..n {
            let (a, b) = (r * n + i, r * n + (i + 1) % n);
            let (c, d) = (b + n, a + n);
            triangles.push([a, b, c]);
            triangles.push([a, c, d]);
        }
    }
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

fn circle(cx: f64, cz: f64, r: f64, n: u32, noise: impl Fn(u32) -> f64) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            let rr = r + noise(i);
            [cx + rr * a.cos(), cz + rr * a.sin()]
        })
        .collect()
}

/// Segments with the defaults and checks what must always hold: every region's corners lie within the
/// distance limit (0.2 mm) of its own cylinder, and no triangle is in two regions.
fn segment(m: &Mesh) -> CylinderSegmentation {
    let s = segment_cylinders(m, &MeshGraph::build(m), CylinderSegmentOptions::default());
    let mut seen = vec![false; m.triangles.len()];
    for r in &s.regions {
        let c = &r.cylinder;
        for &t in &r.triangles {
            assert!(!seen[t as usize], "triangle {t} is in two regions");
            seen[t as usize] = true;
            for &v in &m.triangles[t as usize] {
                let p = m.positions[v as usize].map(f64::from);
                let d = sub(p, c.axis_point);
                let off = length(cross(d, c.axis)) - c.radius;
                assert!(
                    off.abs() <= 0.2 + 1e-6,
                    "a region strays {off} mm from its cylinder"
                );
            }
        }
    }
    s
}

#[test]
fn an_open_tube_is_one_round_region() {
    let m = extrude(&circle(3.0, -2.0, 6.0, 32, |_| 0.0), 15.0, 5);
    let s = segment(&m);
    assert_eq!(
        s.regions.len(),
        1,
        "{:?}",
        s.regions
            .iter()
            .map(|r| r.triangles.len())
            .collect::<Vec<_>>()
    );
    let c = &s.regions[0].cylinder;
    assert_eq!(s.regions[0].triangles.len(), m.triangles.len());
    assert!((c.radius - 6.0).abs() < 1e-4, "{c:?}");
    assert!(
        c.axis[1].abs() > 1.0 - 1e-9
            && (c.axis_point[0] - 3.0).abs() < 1e-4
            && (c.axis_point[2] + 2.0).abs() < 1e-4
    );
    assert!(c.coverage_deg > 340.0);
}

#[test]
fn a_rounded_edge_is_found_and_the_flat_faces_are_not() {
    // A 10 x 10 block profile with one edge rounded at radius 3 (centre (7, 7)), in 12 steps.
    let mut profile = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 7.0]];
    for i in 1..12 {
        let a = std::f64::consts::FRAC_PI_2 * f64::from(i) / 12.0;
        profile.push([7.0 + 3.0 * a.cos(), 7.0 + 3.0 * a.sin()]);
    }
    profile.extend([[7.0, 10.0], [0.0, 10.0]]);
    let m = extrude(&profile, 8.0, 4);
    let s = segment(&m);
    assert_eq!(
        s.regions.len(),
        1,
        "{:?}",
        s.regions
            .iter()
            .map(|r| r.cylinder.radius)
            .collect::<Vec<_>>()
    );
    let r = &s.regions[0];
    assert!((r.cylinder.radius - 3.0).abs() < 1e-3, "{:?}", r.cylinder);
    assert!(r.cylinder.axis[1].abs() > 1.0 - 1e-9);
    assert!(
        (r.cylinder.coverage_deg - 90.0).abs() < 10.0,
        "coverage {}",
        r.cylinder.coverage_deg
    );
    // None of the flat faces' interior triangles (all corners far from the arc) is in it.
    for &t in &r.triangles {
        let mid: Vec<f32> = m.triangles[t as usize]
            .iter()
            .map(|&v| m.positions[v as usize][0] + m.positions[v as usize][2])
            .collect();
        assert!(
            mid.iter().all(|&s| s > 13.0),
            "a flat-face triangle joined the fillet"
        );
    }
}

#[test]
fn two_tubes_give_two_regions_and_a_cube_gives_none() {
    let mut a = extrude(&circle(0.0, 0.0, 2.5, 24, |_| 0.0), 10.0, 3);
    let b = extrude(&circle(25.0, 0.0, 2.5, 24, |_| 0.0), 10.0, 3);
    let base = a.positions.len() as u32;
    a.positions.extend(b.positions);
    a.triangles
        .extend(b.triangles.iter().map(|t| t.map(|v| v + base)));
    let s = segment(&a);
    assert_eq!(s.regions.len(), 2);
    for r in &s.regions {
        assert!((r.cylinder.radius - 2.5).abs() < 1e-4);
    }
    let cube = extrude(&[[0.0, 0.0], [5.0, 0.0], [5.0, 5.0], [0.0, 5.0]], 5.0, 2);
    assert!(
        segment(&cube).regions.is_empty(),
        "flat faces are not round"
    );
}

#[test]
fn a_noisy_tube_is_still_one_region() {
    let noise = |i: u32| 0.03 * (f64::from(i) * 2.3).sin();
    let m = extrude(&circle(0.0, 0.0, 8.0, 40, noise), 20.0, 6);
    let s = segment(&m);
    assert_eq!(s.regions.len(), 1);
    assert!(s.regions[0].triangles.len() as f64 > 0.95 * m.triangles.len() as f64);
    assert!((s.regions[0].cylinder.radius - 8.0).abs() < 0.02);
    let _ = dot([0.0; 3], [0.0; 3]);
}

#[test]
fn a_thin_walled_tubes_end_ring_is_not_part_of_its_side() {
    // Wall 0.1 mm thick: the end ring's triangles lie within 0.2 mm of the outer surface, but they face
    // along the axis, so they must stay out of the round region.
    let (outer, inner, n) = (6.0, 5.9, 32u32);
    let mut m = extrude(&circle(0.0, 0.0, outer, n, |_| 0.0), 10.0, 4);
    let base = m.positions.len() as u32;
    for i in 0..n {
        let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
        m.positions
            .push([(inner * a.cos()) as f32, 0.0, (inner * a.sin()) as f32]);
    }
    let ring: Vec<u32> = (m.triangles.len() as u32..).take(2 * n as usize).collect();
    for i in 0..n {
        let j = (i + 1) % n;
        m.triangles.push([i, base + i, base + j]);
        m.triangles.push([i, base + j, j]);
    }
    let s = segment(&m);
    let side = s
        .regions
        .iter()
        .max_by(|a, b| a.area.total_cmp(&b.area))
        .unwrap();
    assert!(
        (side.cylinder.radius - outer).abs() < 1e-3,
        "{:?}",
        side.cylinder
    );
    assert!(
        side.triangles.iter().all(|t| !ring.contains(t)),
        "an end-ring triangle joined the side"
    );
}
