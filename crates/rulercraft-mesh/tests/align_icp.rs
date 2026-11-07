// SPDX-License-Identifier: MIT OR Apache-2.0
//! align.icp checks: an L-shaped part moved by a known amount must be moved back.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::align::{IcpOptions, Rigid, align_icp};
use rulercraft_mesh::math::{Vec3, add, cross, dot, scale};

/// One face of a box as an n x n grid; `u × v` must point outward.
fn face(m: &mut Mesh, o: Vec3, u: Vec3, v: Vec3, n: u32) {
    let base = m.positions.len() as u32;
    for i in 0..=n {
        for j in 0..=n {
            let p = add(
                o,
                add(
                    scale(u, f64::from(i) / f64::from(n)),
                    scale(v, f64::from(j) / f64::from(n)),
                ),
            );
            m.positions.push(p.map(|x| x as f32));
        }
    }
    for i in 0..n {
        for j in 0..n {
            let a = base + i * (n + 1) + j;
            let (b, c, d) = (a + 1, a + n + 2, a + n + 1);
            m.triangles.push([a, d, c]);
            m.triangles.push([a, c, b]);
        }
    }
}
fn boxy(m: &mut Mesh, lo: Vec3, s: Vec3, n: u32) {
    let (x, y, z) = ([s[0], 0., 0.], [0., s[1], 0.], [0., 0., s[2]]);
    let hi = add(lo, s);
    face(m, lo, y, x, n); // bottom (y × x = -z)
    face(m, [lo[0], lo[1], hi[2]], x, y, n); // top
    face(m, lo, z, y, n); // left (-x)
    face(m, [hi[0], lo[1], lo[2]], y, z, n); // right
    face(m, lo, x, z, n); // front (-y)
    face(m, [lo[0], hi[1], lo[2]], z, x, n); // back
}
fn part() -> Mesh {
    let mut m = Mesh::default();
    boxy(&mut m, [0., 0., 0.], [20., 20., 10.], 6);
    boxy(&mut m, [0., 0., 10.], [10., 10., 10.], 6);
    m
}
fn turned(a: f64, t: Vec3) -> Rigid {
    let (c, s) = (a.cos(), a.sin());
    // About an axis tilted off z so all three turn components matter.
    let k = [0.3f64, 0.5, 0.8];
    let n = (k[0] * k[0] + k[1] * k[1] + k[2] * k[2]).sqrt();
    let k = k.map(|x| x / n);
    let v = 1.0 - c;
    Rigid {
        rotation: [
            [
                c + k[0] * k[0] * v,
                k[0] * k[1] * v - k[2] * s,
                k[0] * k[2] * v + k[1] * s,
            ],
            [
                k[1] * k[0] * v + k[2] * s,
                c + k[1] * k[1] * v,
                k[1] * k[2] * v - k[0] * s,
            ],
            [
                k[2] * k[0] * v - k[1] * s,
                k[2] * k[1] * v + k[0] * s,
                c + k[2] * k[2] * v,
            ],
        ],
        translation: t,
    }
}
fn moved(m: &Mesh, t: &Rigid) -> Mesh {
    let mut o = m.clone();
    o.positions = m
        .positions
        .iter()
        .map(|&p| t.apply(p.map(f64::from)).map(|x| x as f32))
        .collect();
    o
}

#[test]
fn a_part_moved_by_a_known_amount_is_moved_back() {
    let reference = part();
    let t = turned(0.05, [0.6, -0.4, 0.3]); // about 2.9 degrees and under a millimetre
    let scan = moved(&reference, &t);
    let r = align_icp(&scan, &reference, Rigid::IDENTITY, IcpOptions::default()).unwrap();
    assert!(
        r.rms_before > 0.3,
        "the start should be visibly off: {}",
        r.rms_before
    );
    assert!(r.rms_after < 1e-3, "{r:?}");
    for &p in &reference.positions {
        let p = p.map(f64::from);
        let back = r.transform.apply(t.apply(p));
        assert!(
            dot(sub3(back, p), sub3(back, p)).sqrt() < 2e-3,
            "{back:?} vs {p:?}"
        );
    }
}

fn sub3(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[test]
fn an_aligned_scan_stays_put() {
    let p = part();
    let r = align_icp(&p, &p, Rigid::IDENTITY, IcpOptions::default()).unwrap();
    assert!(r.rms_before < 1e-6 && r.rms_after < 1e-6 && r.converged);
    assert!((r.transform.translation[0]).abs() < 1e-6);
}

#[test]
fn moving_both_meshes_the_same_way_gives_the_same_residuals() {
    let (reference, w) = (part(), turned(1.1, [50., -20., 8.]));
    let t = turned(0.04, [0.5, 0.2, -0.3]);
    let scan = moved(&reference, &t);
    let a = align_icp(&scan, &reference, Rigid::IDENTITY, IcpOptions::default()).unwrap();
    let b = align_icp(
        &moved(&scan, &w),
        &moved(&reference, &w),
        Rigid::IDENTITY,
        IcpOptions::default(),
    )
    .unwrap();
    // Beside a box edge two faces tie for closest and either normal may win, so allow 2%.
    assert!(
        (a.rms_before - b.rms_before).abs() < 0.02 * a.rms_before,
        "{} vs {}",
        a.rms_before,
        b.rms_before
    );
    assert!(b.rms_after < 2e-3);
}

#[test]
fn bad_input_is_refused_plainly() {
    let p = part();
    let o = IcpOptions::default();
    assert!(align_icp(&p, &Mesh::default(), Rigid::IDENTITY, o).is_err());
    assert!(align_icp(&Mesh::default(), &p, Rigid::IDENTITY, o).is_err());
    for bad in [
        IcpOptions {
            max_iterations: 0,
            ..o
        },
        IcpOptions {
            tolerance: f64::NAN,
            ..o
        },
        IcpOptions {
            max_pair_distance: 0.0,
            ..o
        },
        IcpOptions { max_points: 3, ..o },
    ] {
        assert!(align_icp(&p, &p, Rigid::IDENTITY, bad).is_err());
    }
    // Everything farther than the pair limit from the reference: nothing to align with.
    let far = moved(&p, &turned(0.0, [500., 0., 0.]));
    assert!(
        align_icp(
            &far,
            &p,
            Rigid::IDENTITY,
            IcpOptions {
                max_pair_distance: 1.0,
                ..o
            }
        )
        .is_err()
    );
    let _ = cross([0.; 3], [0.; 3]);
}
