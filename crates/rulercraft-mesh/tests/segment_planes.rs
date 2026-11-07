// SPDX-License-Identifier: MIT OR Apache-2.0
//! segment.planes checks: shapes whose faces are known exactly (docs/specs/segmentation.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::segment::{PlaneSegmentOptions, segment_planes};

fn mesh(positions: Vec<[f32; 3]>, triangles: Vec<[u32; 3]>) -> Mesh {
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

/// Segments with the defaults and checks what must always hold: every region's vertices lie within the
/// distance limit (0.2 mm) of its own fitted plane, and no triangle is in two regions.
fn segment(m: &Mesh) -> rulercraft_mesh::segment::PlaneSegmentation {
    let s = segment_planes(m, &MeshGraph::build(m), PlaneSegmentOptions::default());
    let mut seen = vec![false; m.triangles.len()];
    for r in &s.regions {
        let worst = r.plane.min_distance.abs().max(r.plane.max_distance.abs());
        assert!(
            worst <= 0.2 + 1e-6,
            "a region strays {worst} mm from its plane"
        );
        for &t in &r.triangles {
            assert!(!seen[t as usize], "triangle {t} is in two regions");
            seen[t as usize] = true;
        }
    }
    s
}

/// An L-shaped prism of height `h`, wound outward: 2 caps and 6 sides, 8 flat faces.
fn l_block(h: f32) -> Mesh {
    let l = [
        [0.0f32, 0.0],
        [3.0, 0.0],
        [3.0, 1.0],
        [1.0, 1.0],
        [1.0, 3.0],
        [0.0, 3.0],
    ];
    let mut p: Vec<[f32; 3]> = l.iter().map(|v| [v[0], v[1], 0.0]).collect();
    p.extend(l.iter().map(|v| [v[0], v[1], h]));
    let mut t = vec![];
    // Caps: a fan from the reflex corner 3, counter-clockwise from above for the top.
    for (a, b) in [(4, 5), (5, 0), (0, 1), (1, 2)] {
        t.push([3 + 6, a + 6, b + 6]);
        t.push([3, b, a]);
    }
    for a in 0..6u32 {
        let b = (a + 1) % 6;
        t.push([a, b, b + 6]);
        t.push([a, b + 6, a + 6]);
    }
    mesh(p, t)
}

#[test]
fn cube_gives_its_six_faces() {
    let c = |i: u32| {
        [
            (i & 1) as f32 * 2.0,
            ((i >> 1) & 1) as f32 * 2.0,
            ((i >> 2) & 1) as f32 * 2.0,
        ]
    };
    let mut p = vec![];
    let mut t = vec![];
    for q in [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        let base = p.len() as u32;
        p.extend(q.iter().map(|&k| c(k)));
        t.push([base, base + 1, base + 2]);
        t.push([base, base + 2, base + 3]);
    }
    // Weld shared corners so it is one closed mesh, as a reader would produce.
    let m = rulercraft_mesh::read_mesh(
        "c.obj",
        obj(&p, &t).as_bytes(),
        rulercraft_mesh::Unit::Mm,
        Default::default(),
    )
    .unwrap();
    let s = segment(&m);
    assert_eq!(s.regions.len(), 6);
    assert_eq!(s.unassigned, 0);
    let mut normals: Vec<[i32; 3]> = s
        .regions
        .iter()
        .map(|r| {
            assert_eq!(r.triangles.len(), 2);
            assert!((r.area - 4.0).abs() < 1e-9);
            assert_eq!(r.plane.sigma, 0.0);
            r.plane.normal.map(|x| x.round() as i32)
        })
        .collect();
    normals.sort();
    assert_eq!(
        normals,
        vec![
            [-1, 0, 0],
            [0, -1, 0],
            [0, 0, -1],
            [0, 0, 1],
            [0, 1, 0],
            [1, 0, 0]
        ]
    );
}

fn obj(p: &[[f32; 3]], t: &[[u32; 3]]) -> String {
    let mut s = String::new();
    for v in p {
        s += &format!("v {} {} {}\n", v[0], v[1], v[2]);
    }
    for f in t {
        s += &format!("f {} {} {}\n", f[0] + 1, f[1] + 1, f[2] + 1);
    }
    s
}

#[test]
fn l_block_gives_eight_faces_with_the_right_areas() {
    let s = segment(&l_block(2.0));
    assert_eq!(
        s.regions.len(),
        8,
        "{:?}",
        s.regions.iter().map(|r| r.area).collect::<Vec<_>>()
    );
    let mut areas: Vec<i64> = s
        .regions
        .iter()
        .map(|r| (r.area * 1000.0).round() as i64)
        .collect();
    areas.sort_unstable();
    // Caps 5 each; sides 3x2 (two), 1x2 (two), 2x2 (two).
    assert_eq!(areas, vec![2000, 2000, 4000, 4000, 5000, 5000, 6000, 6000]);
    assert_eq!(s.regions[0].area, 6.0);
}

struct Noise(u64);
impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }
}

fn grid(n: u32, z: impl Fn(f64, f64) -> f64) -> Mesh {
    let mut p = vec![];
    for y in 0..n {
        for x in 0..n {
            p.push([x as f32, y as f32, z(f64::from(x), f64::from(y)) as f32]);
        }
    }
    let mut t = vec![];
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let a = y * n + x;
            t.push([a, a + 1, a + n]);
            t.push([a + 1, a + n + 1, a + n]);
        }
    }
    mesh(p, t)
}

#[test]
fn a_noisy_plate_is_one_region() {
    let mut noise = Noise(5);
    let noisy: Vec<f64> = (0..400).map(|_| 0.03 * noise.next()).collect();
    let m = grid(20, |x, y| noisy[(y as usize) * 20 + x as usize]);
    let s = segment(&m);
    assert_eq!(
        s.regions[0].triangles.len(),
        m.triangles.len(),
        "{} regions",
        s.regions.len()
    );
    assert!(s.regions[0].plane.normal[2] > 0.999);
}

#[test]
fn a_dome_on_a_plate_stays_out_of_the_plate() {
    let dome = |x: f64, y: f64| {
        let r2 = (x - 15.0).powi(2) + (y - 15.0).powi(2);
        if r2 < 25.0 {
            3.0 * (1.0 - r2 / 25.0)
        } else {
            0.0
        }
    };
    let m = grid(31, dome);
    let s = segment(&m);
    let plate = &s.regions[0];
    let flat: Vec<u32> = (0..m.triangles.len() as u32)
        .filter(|&t| {
            m.triangles[t as usize]
                .iter()
                .all(|&v| m.positions[v as usize][2] == 0.0)
        })
        .collect();
    assert!(plate.triangles.iter().all(|&t| {
        m.triangles[t as usize]
            .iter()
            .all(|&v| m.positions[v as usize][2] <= 0.2)
    }));
    assert_eq!(
        plate.triangles, flat,
        "the plate is exactly the triangles lying on it"
    );
    assert!(s.regions.len() > 1);
}

#[test]
fn small_regions_can_be_dropped_and_an_empty_mesh_is_fine() {
    let m = l_block(2.0);
    let s = segment_planes(
        &m,
        &MeshGraph::build(&m),
        PlaneSegmentOptions {
            min_area: 4.5,
            ..Default::default()
        },
    );
    assert_eq!(s.regions.len(), 4);
    assert_eq!(
        s.unassigned,
        m.triangles.len() - s.regions.iter().map(|r| r.triangles.len()).sum::<usize>()
    );
    let empty = mesh(vec![], vec![]);
    assert!(segment(&empty).regions.is_empty());
}

#[test]
fn a_gentle_step_keeps_its_two_levels_apart() {
    // A plate at z = 0, a 5° ramp, and a plate at z = 1: every triangle is within the 10° angle limit of
    // the first plate, so only the distance limit keeps the upper level out of the lower one.
    let ramp = 5f64.to_radians().tan();
    let m = grid(60, |x, _| ((x - 10.0) * ramp).clamp(0.0, 1.0));
    let s = segment(&m);
    // segment() already checks each region stays within 0.2 mm of its plane; here, the two levels are
    // separate regions, each flat at its own height.
    let lower = s
        .regions
        .iter()
        .position(|r| r.triangles.contains(&0))
        .unwrap();
    let last = m.triangles.len() as u32 - 1;
    let upper = s
        .regions
        .iter()
        .position(|r| r.triangles.contains(&last))
        .unwrap();
    assert_ne!(lower, upper, "the two levels must not be one region");
    let top = s.regions[lower]
        .vertices
        .iter()
        .map(|&v| m.positions[v as usize][2])
        .fold(0.0f32, f32::max);
    // The floor may take in the start of the ramp while staying within 0.2 mm of its own plane (the
    // spec allows that); it must never reach the upper level.
    assert!(top < 0.8, "the floor climbed the ramp to {top}");
    let up = &s.regions[upper].plane;
    assert!(
        up.normal[2] > 0.999 && (up.center[2] - 1.0).abs() < 0.05,
        "{up:?}"
    );
}

/// Opt-in, on a private real scan if present (`cargo test --release -- --ignored real_scan`).
#[test]
#[ignore = "needs fixtures/private (gitignored) and a release build"]
fn real_scan_segments_in_reasonable_time() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/private");
    let Some(path) = std::fs::read_dir(&dir).ok().and_then(|d| {
        d.flatten()
            .flat_map(|e| std::fs::read_dir(e.path()).into_iter().flatten().flatten())
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "ply"))
    }) else {
        eprintln!("skipped: no .ply under fixtures/private");
        return;
    };
    let bytes = std::fs::read(&path).unwrap();
    let start = std::time::Instant::now();
    let m = rulercraft_mesh::read_mesh(
        "scan.ply",
        &bytes,
        rulercraft_mesh::Unit::Mm,
        Default::default(),
    )
    .unwrap();
    let g = MeshGraph::build(&m);
    let built = start.elapsed();
    let s = segment(&m);
    eprintln!(
        "real scan: {} triangles, graph {:?}, segmentation {:?}, {} regions, largest {:.0} mm² ({} triangles), unassigned {}",
        m.triangles.len(),
        built,
        start.elapsed() - built,
        s.regions.len(),
        s.regions[0].area,
        s.regions[0].triangles.len(),
        s.unassigned
    );
    let _ = g;
    assert!(start.elapsed().as_secs() < 30);
}
