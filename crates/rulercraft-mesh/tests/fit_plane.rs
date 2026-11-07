// SPDX-License-Identifier: MIT OR Apache-2.0
//! fit.plane checks: point sets whose plane is known exactly (docs/specs/fit-plane.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, FitError, fit_plane};
use rulercraft_mesh::math::{add, cross, dot, normalize, scale};
use rulercraft_mesh::{Limits, Unit, read_stl};

/// Deterministic uniform noise in [-1, 1).
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

fn grid(n: i32, z: f32) -> Vec<[f32; 3]> {
    let mut p = vec![];
    for i in 0..n {
        for j in 0..n {
            p.push([i as f32, j as f32, z]);
        }
    }
    p
}

fn all(p: &[[f32; 3]]) -> Vec<u32> {
    (0..p.len() as u32).collect()
}

#[test]
fn exact_grid_gives_its_plane_and_spans() {
    let p = grid(11, 5.0);
    let f = fit_plane(&p, None, &all(&p), Clip::Sigma(3)).unwrap();
    assert_eq!(f.normal, [0.0, 0.0, 1.0]);
    assert_eq!(f.center, [5.0, 5.0, 5.0]);
    assert_eq!((f.sigma, f.form_error), (0.0, 0.0));
    assert!((f.span_u - 10.0).abs() < 1e-12 && (f.span_v - 10.0).abs() < 1e-12);
    assert_eq!((f.used, f.offered, f.rounds), (121, 121, 1));
    assert!(dot(f.basis_u, f.normal).abs() < 1e-15 && dot(f.basis_v, f.normal).abs() < 1e-15);
}

#[test]
fn noisy_plane_recovers_normal_and_noise_level() {
    // Uniform noise of half-width a has standard deviation a/√3.
    let a = 0.0173;
    let mut noise = Noise(7);
    let p: Vec<[f32; 3]> = grid(40, 5.0)
        .into_iter()
        .map(|q| [q[0], q[1], q[2] + (a * noise.next()) as f32])
        .collect();
    let f = fit_plane(&p, None, &all(&p), Clip::None).unwrap();
    assert!(f.normal[2] > 1.0 - 1e-6, "{:?}", f.normal);
    assert!(
        (f.sigma - a / 3f64.sqrt()).abs() < 0.001,
        "sigma {}",
        f.sigma
    );
    assert!((f.center[2] - 5.0).abs() < 1e-3);
}

#[test]
fn three_sigma_clipping_ignores_far_outliers() {
    let mut p = grid(20, 5.0);
    let mut noise = Noise(3);
    for q in &mut p {
        q[2] += (0.01 * noise.next()) as f32;
    }
    let good = p.len();
    for i in 0..8 {
        p.push([i as f32 * 2.0, 3.0, 15.0]);
    }
    let clipped = fit_plane(&p, None, &all(&p), Clip::Sigma(3)).unwrap();
    assert_eq!(clipped.used, good);
    assert!(clipped.rounds > 1);
    assert!((clipped.center[2] - 5.0).abs() < 1e-3 && clipped.normal[2] > 1.0 - 1e-6);
    let raw = fit_plane(&p, None, &all(&p), Clip::None).unwrap();
    assert!(
        (raw.center[2] - 5.0).abs() > 0.1,
        "outliers should pull the unclipped fit"
    );
}

#[test]
fn tilted_plane_far_from_the_origin() {
    let n = normalize([1.0, -2.0, 0.5]).unwrap();
    let u = normalize(cross(n, [0.0, 0.0, 1.0])).unwrap();
    let v = cross(n, u);
    let origin = [1.0e5, -2.0e5, 3.0e4];
    let mut p = vec![];
    for i in -15..=15 {
        for j in -10..=10 {
            let q = add(
                origin,
                add(scale(u, f64::from(i)), scale(v, f64::from(j) * 0.5)),
            );
            p.push(q.map(|x| x as f32));
        }
    }
    let f = fit_plane(&p, None, &all(&p), Clip::Sigma(3)).unwrap();
    // f32 positions at 2e5 mm carry about 0.008 mm of rounding; the normal must still be right.
    assert!(dot(f.normal, n).abs() > 1.0 - 1e-6, "{:?}", f.normal);
    assert!(f.sigma < 0.01);
}

#[test]
fn vertex_normals_orient_the_result_outward() {
    let p = grid(5, 0.0);
    let down = vec![[0.0f32, 0.0, -1.0]; p.len()];
    let f = fit_plane(&p, Some(&down), &all(&p), Clip::None).unwrap();
    assert_eq!(f.normal, [0.0, 0.0, -1.0]);
    assert!((dot(cross(f.normal, f.basis_u), f.basis_v) - 1.0).abs() < 1e-15);
}

#[test]
fn cube_face_read_from_stl_fits_exactly() {
    let mut text = String::from("solid c\n");
    let c = |i: u32| [(i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32];
    for q in [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
            text += "facet normal 0 0 0\nouter loop\n";
            for k in t {
                let v = c(k);
                text += &format!("vertex {} {} {}\n", v[0] * 20.0, v[1] * 20.0, v[2] * 20.0);
            }
            text += "endloop\nendfacet\n";
        }
    }
    let mesh = read_stl(text.as_bytes(), Unit::Mm, Limits::default()).unwrap();
    let top: Vec<u32> = (0..mesh.positions.len() as u32)
        .filter(|&i| mesh.positions[i as usize][2] == 20.0)
        .collect();
    assert_eq!(top.len(), 4);
    let f = fit_plane(&mesh.positions, None, &top, Clip::Sigma(3)).unwrap();
    assert_eq!(
        (f.normal, f.center[2], f.sigma),
        ([0.0, 0.0, 1.0], 20.0, 0.0)
    );
    assert!((f.span_u - 20.0).abs() < 1e-9 || (f.span_u - 20.0 * 2f64.sqrt()).abs() < 1e-9);
}

#[test]
fn refusals_say_what_is_wrong() {
    let p = grid(3, 0.0);
    assert_eq!(
        fit_plane(&p, None, &[0, 1], Clip::None),
        Err(FitError::TooFewPoints {
            found: 2,
            needed: 3
        })
    );
    assert_eq!(
        fit_plane(&p, None, &[0, 1, 99], Clip::None),
        Err(FitError::NoSuchVertex {
            index: 99,
            count: 9
        })
    );
    let line: Vec<[f32; 3]> = (0..10).map(|i| [i as f32, 2.0 * i as f32, 0.0]).collect();
    assert!(matches!(
        fit_plane(&line, None, &all(&line), Clip::None),
        Err(FitError::Degenerate(_))
    ));
    let same = vec![[1.0f32, 1.0, 1.0]; 5];
    assert!(matches!(
        fit_plane(&same, None, &all(&same), Clip::None),
        Err(FitError::Degenerate(_))
    ));
    let mut bad = p.clone();
    bad[4][1] = f32::NAN;
    assert_eq!(
        fit_plane(&bad, None, &all(&bad), Clip::None),
        Err(FitError::NotFinite)
    );
    assert_eq!(
        fit_plane(&p, None, &all(&p), Clip::Sigma(4)),
        Err(FitError::BadClip(4))
    );
    assert!(
        FitError::TooFewPoints {
            found: 2,
            needed: 3
        }
        .to_string()
        .contains("at least 3")
    );
}
