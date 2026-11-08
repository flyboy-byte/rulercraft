// SPDX-License-Identifier: MIT OR Apache-2.0
//! engine.parallel checks: every parallel result is bit-for-bit the same for 1 and many threads.
#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::align::{IcpOptions, Rigid, align_icp};
use rulercraft_mesh::deviation::deviation;
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::measure::thickness::{ThicknessOptions, wall_thickness};
use rulercraft_mesh::par;
use rulercraft_mesh::reverse::symmetry_planes;
use rulercraft_mesh::spatial::Bvh;

/// A closed ellipsoid with three different radii and a bump, about 2 * n * n triangles. The bump
/// stops it sliding under ICP and breaks its mirror symmetry along x.
fn blob(n: u32, shift: [f64; 3]) -> Mesh {
    let mut positions = vec![];
    for i in 0..=n {
        let th = std::f64::consts::PI * f64::from(i) / f64::from(n);
        for j in 0..n {
            let ph = std::f64::consts::TAU * f64::from(j) / f64::from(n);
            let (x, y, z) = (th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos());
            let bump = 1.0 + 0.15 * (-((x - 0.7).powi(2) + (y - 0.3).powi(2)) * 8.0).exp();
            positions.push([
                (30.0 * x * bump + shift[0]) as f32,
                (20.0 * y * bump + shift[1]) as f32,
                (12.0 * z * bump + shift[2]) as f32,
            ]);
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

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn map_keeps_item_order_for_every_thread_count() {
    let want: Vec<usize> = (0..10_007).map(|i| i * 3 + 1).collect();
    for k in [1, 2, 3, 7, 16, 64] {
        assert_eq!(
            par::with_threads(k, || par::map(10_007, |i| i * 3 + 1)),
            want,
            "{k} threads"
        );
    }
    assert!(par::map(0, |i| i).is_empty());
}

#[test]
fn with_threads_sets_and_restores_the_count() {
    let before = par::threads();
    par::with_threads(3, || {
        assert_eq!(par::threads(), 3);
        par::with_threads(0, || assert_eq!(par::threads(), 1));
        assert_eq!(par::threads(), 3);
    });
    assert_eq!(par::threads(), before);
    let caught = std::panic::catch_unwind(|| par::with_threads(5, || panic!("boom")));
    assert!(caught.is_err());
    assert_eq!(par::threads(), before, "restored after a panic too");
}

#[test]
fn a_panic_in_a_worker_reaches_the_caller() {
    let caught = std::panic::catch_unwind(|| {
        par::with_threads(4, || {
            par::map(8_000, |i| {
                if i == 6_000 {
                    panic!("item {i}");
                }
                i
            })
        })
    });
    assert!(caught.is_err());
}

#[test]
fn deviation_icp_and_symmetry_are_identical_for_one_and_many_threads() {
    let reference = blob(60, [0.0; 3]);
    let scan = blob(50, [0.4, -0.3, 0.2]);
    assert!(
        scan.positions.len() > 2_000,
        "enough vertices to split over threads"
    );
    let opts = IcpOptions {
        max_points: 5_000,
        ..IcpOptions::default()
    };
    let run = |k: usize| {
        par::with_threads(k, || {
            (
                deviation(&scan, &reference, 0.1).unwrap(),
                align_icp(&scan, &reference, Rigid::IDENTITY, opts).unwrap(),
                symmetry_planes(&scan, 0.05).unwrap(),
            )
        })
    };
    let (d1, i1, s1) = run(1);
    for k in [2, 5, 13] {
        let (dk, ik, sk) = run(k);
        assert_eq!(bits(&dk.distances), bits(&d1.distances), "{k} threads");
        assert_eq!(dk.stats, d1.stats);
        assert_eq!(ik, i1, "{k} threads");
        assert_eq!(sk, s1, "{k} threads");
    }
    // The answers are also real: ICP moves the scan back by the shift it was given.
    let t = i1.transform.translation;
    assert!(
        (t[0] + 0.4).abs() < 0.05 && (t[1] - 0.3).abs() < 0.05 && (t[2] + 0.2).abs() < 0.05,
        "{i1:?}"
    );
}

#[test]
fn the_tree_and_thickness_do_not_depend_on_the_thread_count() {
    // More triangles than one subtree is worth a thread, so the build really splits.
    let m = blob(230, [0.0; 3]);
    assert!(m.triangles.len() > 80_000);
    let graph = MeshGraph::build(&m);
    let sel: Vec<u32> = (0..m.positions.len() as u32).step_by(37).collect();
    assert!(sel.len() > 1_000, "enough rays to split over threads");
    let run = |k: usize| {
        par::with_threads(k, || {
            let tree = Bvh::new(&m);
            let probes: Vec<_> = (0..300)
                .map(|i| {
                    let f = f64::from(i);
                    let p = [
                        40.0 * (f * 0.7).sin(),
                        30.0 * (f * 1.3).cos(),
                        20.0 * (f * 0.37).sin(),
                    ];
                    let c = tree.closest(p).unwrap();
                    (c.point.map(f64::to_bits), c.triangle, c.distance2.to_bits())
                })
                .collect::<Vec<_>>();
            let options = ThicknessOptions {
                max_triangle_tests: usize::MAX,
                ..ThicknessOptions::default()
            };
            let t = wall_thickness(&m, &graph.vertex_normals, &sel, options).unwrap();
            (tree.node_count(), probes, t)
        })
    };
    let one = run(1);
    assert!(one.2.iter().any(|s| s.distance.is_some()), "some rays hit");
    for k in [2, 3, 8, 12] {
        assert_eq!(run(k), one, "{k} threads");
    }
}

/// Speed check, run by hand in release: `cargo test --release -p rulercraft-mesh --test parallel --
/// --ignored --nocapture`. Deviation of a 1-million-vertex mesh on one thread and on all of them.
#[test]
#[ignore = "slow; run by hand in release"]
fn deviation_is_faster_on_many_threads() {
    let reference = blob(700, [0.0; 3]);
    let scan = blob(700, [0.05, 0.0, 0.0]);
    let time = |k: usize| {
        let start = std::time::Instant::now();
        let d = par::with_threads(k, || deviation(&scan, &reference, 0.1).unwrap());
        (start.elapsed(), d)
    };
    let (one, d1) = time(1);
    let n = par::threads();
    let (all, dn) = time(n);
    assert_eq!(bits(&d1.distances), bits(&dn.distances));
    println!(
        "{} vertices against {} triangles: 1 thread {one:?}, {n} threads {all:?}, {:.1}x",
        scan.positions.len(),
        reference.triangles.len(),
        one.as_secs_f64() / all.as_secs_f64()
    );
    // Measured 2026-11-07, 12 threads: 1.7x end to end. Building the tree and the statistics' sort
    // still run on one thread, so this is not yet the speed-up engine.parallel needs.
    if n >= 4 {
        assert!(one.as_secs_f64() / all.as_secs_f64() > 2.0);
    }
}
