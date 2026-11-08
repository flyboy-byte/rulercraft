// SPDX-License-Identifier: MIT OR Apache-2.0
//! engine.bvh checks: the shared tree must give exactly the answers of testing every triangle.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::math::{Vec3, add, cross, dot, normalize, scale, sub};
use rulercraft_mesh::spatial::{Bvh, closest_point_on_triangle};

/// A small seeded generator so every run sees the same shapes.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

/// Random triangles scattered in a 100 mm box, sizes from tiny to large, plus a grid of
/// axis-aligned triangles so rays parallel to an axis meet flat boxes.
fn soup(seed: u64, n: usize) -> Mesh {
    let mut r = Lcg(seed);
    let mut positions = vec![];
    let mut triangles = vec![];
    for i in 0..n {
        let c = [r.range(-50., 50.), r.range(-50., 50.), r.range(-50., 50.)];
        let s = if i % 7 == 0 { 20.0 } else { 3.0 };
        let base = positions.len() as u32;
        for _ in 0..3 {
            positions.push([
                (c[0] + r.range(-s, s)) as f32,
                (c[1] + r.range(-s, s)) as f32,
                (c[2] + r.range(-s, s)) as f32,
            ]);
        }
        triangles.push([base, base + 1, base + 2]);
    }
    for i in 0..10 {
        for j in 0..10 {
            let (x, y, z) = (-45.0 + 9.0 * f64::from(i), -45.0 + 9.0 * f64::from(j), 10.0);
            let base = positions.len() as u32;
            positions.extend([
                [x as f32, y as f32, z as f32],
                [(x + 9.0) as f32, y as f32, z as f32],
                [x as f32, (y + 9.0) as f32, z as f32],
            ]);
            triangles.push([base, base + 1, base + 2]);
        }
    }
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

fn corners(m: &Mesh, t: usize) -> [Vec3; 3] {
    m.triangles[t].map(|i| m.positions[i as usize].map(f64::from))
}

/// Two-sided ray-triangle test with a small edge allowance, as callers write them.
fn ray_hit(o: Vec3, d: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f64> {
    let (e1, e2) = (sub(b, a), sub(c, a));
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-15 {
        return None;
    }
    let s = sub(o, a);
    let u = dot(s, p) / det;
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    if u < -1e-12 || v < -1e-12 || u + v > 1.0 + 1e-12 {
        return None;
    }
    Some(dot(e2, q) / det)
}

#[test]
fn closest_point_matches_testing_every_triangle() {
    let m = soup(1, 1200);
    let tree = Bvh::new(&m);
    assert_eq!(tree.len(), m.triangles.len());
    let mut r = Lcg(2);
    for _ in 0..600 {
        let p = [r.range(-80., 80.), r.range(-80., 80.), r.range(-80., 80.)];
        let got = tree.closest(p).unwrap();
        let best = (0..m.triangles.len())
            .map(|t| {
                let [a, b, c] = corners(&m, t);
                let q = closest_point_on_triangle(p, a, b, c);
                dot(sub(p, q), sub(p, q))
            })
            .fold(f64::INFINITY, f64::min);
        assert_eq!(got.distance2, best, "at {p:?}");
        let [a, b, c] = corners(&m, got.triangle);
        assert_eq!(got.point, closest_point_on_triangle(p, a, b, c));
    }
}

#[test]
fn nearest_ray_hit_matches_testing_every_triangle() {
    let m = soup(3, 1200);
    let tree = Bvh::new(&m);
    let mut r = Lcg(4);
    let mut hits = 0;
    for k in 0..1000 {
        let o = [r.range(-60., 60.), r.range(-60., 60.), r.range(-60., 60.)];
        // Every fourth ray runs along an axis, exercising the parallel-slab case.
        let d = if k % 4 == 0 {
            let mut d = [0.0; 3];
            d[k % 3] = if k % 8 == 0 { 1.0 } else { -1.0 };
            d
        } else {
            normalize([r.range(-1., 1.), r.range(-1., 1.), r.range(-1., 1.)]).unwrap()
        };
        let (t_min, t_max) = (1e-6, r.range(5., 150.));
        let got = tree.ray_nearest(o, d, t_min, t_max, 1e-6, |_, c| ray_hit(o, d, c));
        let want = (0..m.triangles.len())
            .filter_map(|t| {
                ray_hit(o, d, corners(&m, t))
                    .filter(|x| *x > t_min && *x <= t_max)
                    .map(|x| (x, t))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        assert_eq!(got, want, "ray {k} from {o:?} along {d:?}");
        hits += usize::from(want.is_some());
    }
    assert!(
        hits > 200,
        "too few rays hit anything ({hits}) for this to test much"
    );
}

#[test]
fn rays_from_inside_a_flat_box_and_axis_rays_on_a_face_are_found() {
    // One flat square in z = 0: its box has no thickness. A ray straight down onto it, and a ray
    // starting exactly on it, both have to reach the triangle.
    let m = Mesh {
        positions: vec![[0., 0., 0.], [10., 0., 0.], [0., 10., 0.]],
        triangles: vec![[0, 1, 2]],
        degenerate_dropped: 0,
    };
    let tree = Bvh::new(&m);
    let down = [0.0, 0.0, -1.0];
    let o = [2.0, 2.0, 5.0];
    assert_eq!(
        tree.ray_nearest(o, down, 0.0, 100.0, 0.0, |_, c| ray_hit(o, down, c)),
        Some((5.0, 0))
    );
    let missing = [20.0, 2.0, 5.0];
    assert_eq!(
        tree.ray_nearest(missing, down, 0.0, 100.0, 0.0, |_, c| ray_hit(
            missing, down, c
        )),
        None
    );
    // Beyond the range: no hit, even though the triangle is on the ray.
    assert_eq!(
        tree.ray_nearest(o, down, 0.0, 4.0, 0.0, |_, c| ray_hit(o, down, c)),
        None
    );
}

#[test]
fn equal_distances_keep_the_lowest_triangle_number() {
    // Twenty triangles in z = 0 that all cover (1, 1), each shifted further toward -x than the last, so
    // the tree meets the highest numbers first. Every one is hit at the same distance.
    let mut m = Mesh {
        positions: vec![],
        triangles: vec![],
        degenerate_dropped: 0,
    };
    for i in 0..20u32 {
        let s = i as f32;
        m.positions
            .extend([[-1. - s, -1., 0.], [30. - s, -1., 0.], [-1. - s, 30., 0.]]);
        m.triangles.push([3 * i, 3 * i + 1, 3 * i + 2]);
    }
    let tree = Bvh::new(&m);
    let (o, d) = ([1.0, 1.0, 3.0], [0.0, 0.0, -1.0]);
    let got = tree.ray_nearest(o, d, 0.0, 10.0, 0.0, |_, c| ray_hit(o, d, c));
    assert_eq!(got, Some((3.0, 0)));
}

#[test]
fn a_hit_just_outside_a_triangle_is_kept_when_the_test_allows_it() {
    // The ray passes 5e-12 mm outside the triangle's edge x = 0, inside the hit test's allowance (u >=
    // -1e-12 of a 10 mm edge). The box must be widened by `pad` or the tree would never test it.
    let m = Mesh {
        positions: vec![[0., 0., 0.], [10., 0., 0.], [0., 10., 0.]],
        triangles: vec![[0, 1, 2]],
        degenerate_dropped: 0,
    };
    let tree = Bvh::new(&m);
    let (o, d) = ([-5e-12, 2.0, 5.0], [0.0, 0.0, -1.0]);
    let want = ray_hit(o, d, corners(&m, 0)).map(|t| (t, 0));
    assert!(want.is_some(), "the hit test itself accepts this ray");
    assert_eq!(
        tree.ray_nearest(o, d, 0.0, 100.0, 1e-9, |_, c| ray_hit(o, d, c)),
        want
    );
}

#[test]
fn an_empty_mesh_gives_an_empty_tree() {
    let m = Mesh {
        positions: vec![],
        triangles: vec![],
        degenerate_dropped: 0,
    };
    let tree = Bvh::new(&m);
    assert!(tree.is_empty());
    assert!(tree.closest([0.0; 3]).is_none());
    assert!(
        tree.ray_nearest([0.0; 3], [1.0, 0.0, 0.0], 0.0, 1.0, 0.0, |_, _| Some(0.5))
            .is_none()
    );
}

/// A closed sphere of about 2 * n * n triangles.
fn sphere(n: u32, r: f64) -> Mesh {
    let mut positions = vec![];
    for i in 0..=n {
        let th = std::f64::consts::PI * f64::from(i) / f64::from(n);
        for j in 0..n {
            let ph = std::f64::consts::TAU * f64::from(j) / f64::from(n);
            let p = scale([th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()], r);
            positions.push(p.map(|v| v as f32));
        }
    }
    let mut triangles = vec![];
    for i in 0..n {
        for j in 0..n {
            let (a, b) = (i * n + j, i * n + (j + 1) % n);
            let (c, d) = (a + n, b + n);
            triangles.push([a, c, b]);
            triangles.push([b, c, d]);
        }
    }
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

/// Speed check, run by hand in release: `cargo test --release -p rulercraft-mesh --test spatial --
/// --ignored --nocapture`. Prints the time per query for the tree and for testing every triangle on a
/// 5-million-triangle sphere, and fails unless the tree is at least 100 times faster.
#[test]
#[ignore = "slow; run by hand in release"]
fn the_tree_is_many_times_faster_on_five_million_triangles() {
    let m = sphere(1600, 50.0);
    assert!(m.triangles.len() >= 5_000_000);
    let start = std::time::Instant::now();
    let tree = Bvh::new(&m);
    let build = start.elapsed();
    let mut r = Lcg(9);
    let queries: Vec<Vec3> = (0..20_000)
        .map(|_| [r.range(-60., 60.), r.range(-60., 60.), r.range(-60., 60.)])
        .collect();
    let start = std::time::Instant::now();
    let mut sum = 0.0;
    for &p in &queries {
        sum += tree.closest(p).unwrap().distance2;
    }
    let per_tree = start.elapsed().as_secs_f64() / queries.len() as f64;
    let start = std::time::Instant::now();
    for &p in &queries[..5] {
        let best = (0..m.triangles.len())
            .map(|t| {
                let [a, b, c] = corners(&m, t);
                let q = closest_point_on_triangle(p, a, b, c);
                dot(sub(p, q), sub(p, q))
            })
            .fold(f64::INFINITY, f64::min);
        assert_eq!(best, tree.closest(p).unwrap().distance2);
    }
    let per_brute = start.elapsed().as_secs_f64() / 5.0;
    println!(
        "{} triangles: build {build:?}, tree {:.2} us/query, every triangle {:.1} ms/query, {:.0}x ({sum:.1})",
        m.triangles.len(),
        per_tree * 1e6,
        per_brute * 1e3,
        per_brute / per_tree
    );
    assert!(per_brute / per_tree > 100.0);
    let _ = add([0.0; 3], [0.0; 3]);
}

#[test]
fn parallel_rays_on_box_edges_hit_with_either_sign_of_zero() {
    let mesh = Mesh {
        positions: vec![[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
        triangles: vec![[0, 1, 2]],
        degenerate_dropped: 0,
    };
    let tree = Bvh::new(&mesh);
    for x in [0.0, 1.0] {
        let o = [x, 0.0, 0.0];
        for zero in [0.0, -0.0] {
            let d = [zero, 0.0, 1.0];
            assert_eq!(
                tree.ray_nearest(o, d, 0.0, 2.0, 0.0, |_, c| ray_hit(o, d, c)),
                Some((1.0, 0)),
                "x={x}, sign={}",
                zero.is_sign_negative()
            );
        }
    }
    // A nonzero subnormal can have an infinite reciprocal; it still enters the box.
    let o = [1.0, 0.0, 0.0];
    let d = [-1e-320, 0.0, 1.0];
    assert_eq!(
        tree.ray_nearest(o, d, 0.0, 2.0, 0.0, |_, c| ray_hit(o, d, c)),
        Some((1.0, 0))
    );
    let o = [-0.01, 0.0, 0.0];
    let d = [-0.0, 0.0, 1.0];
    assert_eq!(
        tree.ray_nearest(o, d, 0.0, 2.0, 0.0, |_, c| ray_hit(o, d, c)),
        None
    );
}
