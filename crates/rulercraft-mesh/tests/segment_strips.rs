// SPDX-License-Identifier: MIT OR Apache-2.0
//! segment.strips checks: coarsely faceted round surfaces whose true cylinder is known exactly.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::math::{cross, length, sub};
use rulercraft_mesh::segment::{
    PlaneSegmentOptions, StripJoinOptions, StripSurface, join_strips, segment_planes,
};

/// Extrudes a closed profile in the x-z plane along y from 0 to `depth`, open at both ends.
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

fn polygon(cx: f64, cz: f64, r: f64, n: u32, noise: impl Fn(u32) -> f64) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            let rr = r + noise(i);
            [cx + rr * a.cos(), cz + rr * a.sin()]
        })
        .collect()
}

/// Flat regions with the defaults, then joined strips with the defaults; checks what must always hold:
/// every surface's vertices lie within 0.2 mm of its cylinder and no strip is in two surfaces.
fn join(m: &Mesh) -> Vec<StripSurface> {
    let g = MeshGraph::build(m);
    let planes = segment_planes(m, &g, PlaneSegmentOptions::default());
    let s = join_strips(m, &g, &planes, StripJoinOptions::default());
    let mut seen = vec![false; planes.regions.len()];
    for surface in &s {
        let c = &surface.cylinder;
        for &r in &surface.strips {
            assert!(!seen[r], "strip {r} is in two surfaces");
            seen[r] = true;
        }
        for &v in &surface.vertices {
            let p = m.positions[v as usize].map(f64::from);
            let off = length(cross(sub(p, c.axis_point), c.axis)) - c.radius;
            assert!(
                off.abs() <= 0.2 + 1e-6,
                "a surface strays {off} mm from its cylinder"
            );
        }
    }
    s
}

#[test]
fn a_twelve_sided_tube_is_one_round_surface() {
    // Facets 30° apart: each is its own flat region, and their middles sag 0.2 mm inside the circle.
    let m = extrude(&polygon(3.0, -2.0, 6.0, 12, |_| 0.0), 15.0, 3);
    let s = join(&m);
    assert_eq!(
        s.len(),
        1,
        "{:?}",
        s.iter().map(|s| s.strips.len()).collect::<Vec<_>>()
    );
    let c = &s[0].cylinder;
    assert_eq!(s[0].strips.len(), 12);
    assert_eq!(s[0].triangles.len(), m.triangles.len());
    assert!((c.radius - 6.0).abs() < 1e-4, "{c:?}");
    assert!(c.axis[1].abs() > 1.0 - 1e-9);
    assert!((c.axis_point[0] - 3.0).abs() < 1e-4 && (c.axis_point[2] + 2.0).abs() < 1e-4);
    assert!(c.coverage_deg > 300.0, "coverage {}", c.coverage_deg);
}

#[test]
fn a_coarse_fillet_is_joined_and_the_flat_faces_are_not() {
    // A 10 x 10 block profile with one edge rounded at radius 3 (centre (7, 7)) in four 22.5° steps:
    // the flat faces meet the fillet at 22.5°, inside the turn limit, but their far corners are off it.
    let mut profile = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 7.0]];
    for i in 1..4 {
        let a = std::f64::consts::FRAC_PI_2 * f64::from(i) / 4.0;
        profile.push([7.0 + 3.0 * a.cos(), 7.0 + 3.0 * a.sin()]);
    }
    profile.extend([[7.0, 10.0], [0.0, 10.0]]);
    let m = extrude(&profile, 8.0, 2);
    let s = join(&m);
    assert_eq!(
        s.len(),
        1,
        "{:?}",
        s.iter().map(|s| s.cylinder.radius).collect::<Vec<_>>()
    );
    let c = &s[0].cylinder;
    assert_eq!(s[0].strips.len(), 4);
    assert!((c.radius - 3.0).abs() < 1e-4, "{c:?}");
    assert!(
        (c.coverage_deg - 90.0).abs() < 5.0,
        "coverage {}",
        c.coverage_deg
    );
    for &v in &s[0].vertices {
        let p = m.positions[v as usize];
        assert!(
            p[0] + p[2] > 13.0,
            "a flat face's far corner joined the fillet"
        );
    }
}

#[test]
fn two_coarse_tubes_give_two_surfaces() {
    let mut a = extrude(&polygon(0.0, 0.0, 4.0, 16, |_| 0.0), 10.0, 2);
    let b = extrude(&polygon(30.0, 0.0, 2.0, 10, |_| 0.0), 6.0, 1);
    let base = a.positions.len() as u32;
    a.positions.extend(b.positions);
    a.triangles
        .extend(b.triangles.iter().map(|t| t.map(|v| v + base)));
    let mut radii: Vec<f64> = join(&a).iter().map(|s| s.cylinder.radius).collect();
    radii.sort_by(f64::total_cmp);
    assert_eq!(radii.len(), 2, "{radii:?}");
    assert!(
        (radii[0] - 2.0).abs() < 1e-4 && (radii[1] - 4.0).abs() < 1e-4,
        "{radii:?}"
    );
}

#[test]
fn a_noisy_coarse_tube_is_still_one_surface() {
    // Every vertex is moved radially by its own amount (up to 0.08 mm), so no two strips are exact.
    let mut m = extrude(&polygon(0.0, 0.0, 8.0, 14, |_| 0.0), 20.0, 4);
    for (k, p) in m.positions.iter_mut().enumerate() {
        let e = 0.08 * (k as f32 * 2.3).sin();
        let r = p[0].hypot(p[2]);
        p[0] *= 1.0 + e / r;
        p[2] *= 1.0 + e / r;
    }
    let s = join(&m);
    assert_eq!(
        s.len(),
        1,
        "{:?}",
        s.iter().map(|s| s.strips.len()).collect::<Vec<_>>()
    );
    assert_eq!(s[0].strips.len(), 14);
    assert!(
        (s[0].cylinder.radius - 8.0).abs() < 0.05,
        "{:?}",
        s[0].cylinder
    );
}

#[test]
fn prisms_with_few_sides_stay_flat() {
    // Every corner of a regular prism lies on a circle, so only the turn limit keeps these flat.
    for sides in [3, 4, 6, 8] {
        let m = extrude(&polygon(0.0, 0.0, 5.0, sides, |_| 0.0), 10.0, 2);
        let s = join(&m);
        assert!(
            s.is_empty(),
            "a {sides}-sided prism was called round: {:?}",
            s[0].cylinder
        );
    }
}

/// A plate in the x-z plane meshed as a grid, bent about lines parallel to y: `panels` gives each
/// panel's width (mm) and the turn (degrees) before it. Grid points inside each panel are vertices too.
fn bent_plate(panels: &[(f64, f64)], depth: f64) -> Mesh {
    let mut profile = vec![[0.0, 0.0]];
    let (mut x, mut z, mut heading) = (0.0f64, 0.0f64, 0.0f64);
    for &(width, turn) in panels {
        heading += turn.to_radians();
        for _ in 0..4 {
            x += width / 4.0 * heading.cos();
            z += width / 4.0 * heading.sin();
            profile.push([x, z]);
        }
    }
    // An open strip, not a closed loop: extrude, then drop the quads that close the profile.
    let n = profile.len() as u32;
    let mut m = extrude(&profile, depth, 4);
    m.triangles
        .retain(|t| !t.iter().any(|&v| v % n == n - 1) || !t.iter().any(|&v| v % n == 0));
    m
}

#[test]
fn a_bent_plate_with_wide_flat_panels_is_not_round() {
    // Three 10 mm panels 20° apart: neighbours within the turn limit, and the creases lie on a circle,
    // but the panels' inner grid points sit far off any cylinder through them.
    let m = bent_plate(&[(10.0, 0.0), (10.0, 20.0), (10.0, 20.0)], 10.0);
    let g = MeshGraph::build(&m);
    assert_eq!(
        segment_planes(&m, &g, PlaneSegmentOptions::default())
            .regions
            .len(),
        3
    );
    assert!(join(&m).is_empty(), "flat panels were called round");
}

#[test]
fn a_shallow_arc_below_the_coverage_limit_is_dropped() {
    // Three 6° strips of a radius-100 arc (18° in all), closed through the centre.
    let mut profile: Vec<[f64; 2]> = (0..4)
        .map(|i| {
            let a = (6.0 * f64::from(i)).to_radians();
            [100.0 * a.cos(), 100.0 * a.sin()]
        })
        .collect();
    profile.push([0.0, 0.0]);
    let m = extrude(&profile, 10.0, 2);
    assert!(
        join(&m).is_empty(),
        "an 18° arc passed the 20° coverage limit"
    );
    let g = MeshGraph::build(&m);
    let planes = segment_planes(&m, &g, PlaneSegmentOptions::default());
    let loose = StripJoinOptions {
        min_coverage_deg: 0.0,
        ..Default::default()
    };
    let s = join_strips(&m, &g, &planes, loose);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].strips.len(), 3);
    assert!(
        (s[0].cylinder.radius - 100.0).abs() < 1e-3,
        "{:?}",
        s[0].cylinder
    );
}

/// Opt-in, on a private real scan if present (`cargo test --release -- --ignored real_scan`).
#[test]
#[ignore = "needs fixtures/private (gitignored) and a release build"]
fn real_scan_joins_strips_in_reasonable_time() {
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
    let m = rulercraft_mesh::read_mesh(
        "scan.ply",
        &bytes,
        rulercraft_mesh::Unit::Mm,
        Default::default(),
    )
    .unwrap();
    let g = MeshGraph::build(&m);
    let planes = segment_planes(&m, &g, PlaneSegmentOptions::default());
    let start = std::time::Instant::now();
    let s = join_strips(&m, &g, &planes, StripJoinOptions::default());
    let took = start.elapsed();
    eprintln!(
        "real scan: {} flat regions, join {took:?}, {} surfaces, largest {:?}",
        planes.regions.len(),
        s.len(),
        s.first()
            .map(|s| (s.strips.len(), s.cylinder.radius, s.cylinder.coverage_deg))
    );
    assert!(took.as_secs_f64() < 30.0, "joining took {took:?}");
}
