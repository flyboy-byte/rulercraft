// SPDX-License-Identifier: MIT OR Apache-2.0
//! The SolidCraft rebuild on scan-like meshes: a part described exactly (a signed distance), meshed the way a
//! scanner's software does (surface nets on a grid: dense, even triangles, edges softened by about a cell), turned
//! off the axes, with noise on every vertex. The sizes are known exactly, so the rebuild is checked against them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::solidcraft::design;
use serde_json::Value;

type V3 = [f64; 3];

/// The turn the parts are scanned at: `RULERCRAFT_SCAN_TURN` (degrees) when set, for trying other poses by hand.
fn turn_deg() -> f64 {
    std::env::var("RULERCRAFT_SCAN_TURN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(47.0)
}

/// A deterministic noise in [-1, 1] for an index (a hash, not a random generator, so runs are identical).
fn hash_unit(i: u64) -> f64 {
    let mut x = i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// Surface nets: one vertex per grid cell the surface crosses (the mean of where it crosses the cell's edges), one
/// quad (two triangles) per grid edge it crosses, wound so the normal points to where `sdf` is positive.
fn surface_nets(sdf: &dyn Fn(V3) -> f64, lo: V3, hi: V3, cell: f64) -> Mesh {
    let n = [0, 1, 2].map(|k| ((hi[k] - lo[k]) / cell).ceil() as usize + 1);
    let at = |i: usize, j: usize, k: usize| {
        [
            lo[0] + i as f64 * cell,
            lo[1] + j as f64 * cell,
            lo[2] + k as f64 * cell,
        ]
    };
    let idx = |i: usize, j: usize, k: usize| (i * n[1] + j) * n[2] + k;
    let mut value = vec![0.0; n[0] * n[1] * n[2]];
    for i in 0..n[0] {
        for j in 0..n[1] {
            for k in 0..n[2] {
                value[idx(i, j, k)] = sdf(at(i, j, k));
            }
        }
    }
    let corners = [
        [0, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [1, 1, 0],
        [0, 0, 1],
        [1, 0, 1],
        [0, 1, 1],
        [1, 1, 1],
    ];
    let edges = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let cells = [n[0] - 1, n[1] - 1, n[2] - 1];
    let cidx = |i: usize, j: usize, k: usize| (i * cells[1] + j) * cells[2] + k;
    let mut vertex = vec![u32::MAX; cells[0] * cells[1] * cells[2]];
    let mut positions: Vec<[f32; 3]> = Vec::new();
    for i in 0..cells[0] {
        for j in 0..cells[1] {
            for k in 0..cells[2] {
                let c: Vec<(V3, f64)> = corners
                    .iter()
                    .map(|d| {
                        (
                            at(i + d[0], j + d[1], k + d[2]),
                            value[idx(i + d[0], j + d[1], k + d[2])],
                        )
                    })
                    .collect();
                let (mut sum, mut count) = ([0.0; 3], 0.0);
                for &(a, b) in &edges {
                    let ((pa, va), (pb, vb)) = (c[a], c[b]);
                    if (va < 0.0) != (vb < 0.0) {
                        let t = va / (va - vb);
                        for q in 0..3 {
                            sum[q] += pa[q] + t * (pb[q] - pa[q]);
                        }
                        count += 1.0;
                    }
                }
                if count > 0.0 {
                    vertex[cidx(i, j, k)] = positions.len() as u32;
                    positions.push(sum.map(|s| (s / count) as f32));
                }
            }
        }
    }
    let mut triangles = Vec::new();
    // Each grid edge along axis `a` with a sign change: the four cells around it.
    for a in 0..3 {
        let (b, c) = ((a + 1) % 3, (a + 2) % 3);
        for i in 0..n[0] {
            for j in 0..n[1] {
                for k in 0..n[2] {
                    let p = [i, j, k];
                    if p[a] + 1 >= n[a]
                        || p[b] == 0
                        || p[c] == 0
                        || p[b] > cells[b]
                        || p[c] > cells[c]
                    {
                        continue;
                    }
                    let mut q = p;
                    q[a] += 1;
                    let (v0, v1) = (value[idx(p[0], p[1], p[2])], value[idx(q[0], q[1], q[2])]);
                    if (v0 < 0.0) == (v1 < 0.0) {
                        continue;
                    }
                    let cell_of = |db: usize, dc: usize| {
                        let mut r = p;
                        r[b] -= db;
                        r[c] -= dc;
                        vertex[cidx(r[0], r[1], r[2])]
                    };
                    let quad = [cell_of(1, 1), cell_of(0, 1), cell_of(0, 0), cell_of(1, 0)];
                    if quad.contains(&u32::MAX) {
                        continue;
                    }
                    // Inside to outside along +a: wind counter-clockwise seen from +a.
                    let [w, x, y, z] = if v0 < 0.0 {
                        quad
                    } else {
                        [quad[3], quad[2], quad[1], quad[0]]
                    };
                    triangles.push([w, x, y]);
                    triangles.push([w, y, z]);
                }
            }
        }
    }
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

fn boxd(p: V3, lo: V3, hi: V3) -> f64 {
    let q = [0, 1, 2].map(|k| (lo[k] - p[k]).max(p[k] - hi[k]));
    let out = q.iter().map(|v| v.max(0.0).powi(2)).sum::<f64>().sqrt();
    out + q[0].max(q[1]).max(q[2]).min(0.0)
}

/// A round bar along z: centre `(x, y)`, radius `r`, from `z0` to `z1`.
fn bar(p: V3, x: f64, y: f64, r: f64, z0: f64, z1: f64) -> f64 {
    let d = [(p[0] - x).hypot(p[1] - y) - r, (z0 - p[2]).max(p[2] - z1)];
    d[0].max(d[1]).min(0.0) + d[0].max(0.0).hypot(d[1].max(0.0))
}

/// The test part, in its own frame: a 60 x 40 x 8 plate; a Ø6 through hole at (15, 12); a Ø4 hole 5 deep from the top
/// at (48, 30); a Ø10 boss 5 tall on the top at (42, 12); a 16 x 10 pocket 3 deep from (6, 24).
fn part(p: V3) -> f64 {
    let plate = boxd(p, [0.0, 0.0, 0.0], [60.0, 40.0, 8.0]);
    let boss = bar(p, 42.0, 12.0, 5.0, 4.0, 13.0);
    let solid = plate.min(boss);
    let cuts = bar(p, 15.0, 12.0, 3.0, -1.0, 9.0)
        .min(bar(p, 48.0, 30.0, 2.0, 3.0, 9.0))
        .min(boxd(p, [6.0, 24.0, 5.0], [22.0, 34.0, 9.0]));
    solid.max(-cuts)
}

/// A cone along z: radius `r0` at `z0` widening to `r1` at `z1` (a countersink's shape), closed at both ends.
fn cone(p: V3, x: f64, y: f64, r0: f64, z0: f64, r1: f64, z1: f64) -> f64 {
    let rho = (p[0] - x).hypot(p[1] - y);
    let along = (rho - r0 - (r1 - r0) * (p[2] - z0) / (z1 - z0))
        / (1.0 + ((r1 - r0) / (z1 - z0)).powi(2)).sqrt();
    along.max(z0 - p[2]).max(p[2] - z1)
}

/// The second test part: a 50 x 30 x 8 plate; its top edge along y = 0 bevelled 2 across and 1 down; its top edge
/// along x = 50 rounded R2; a Ø5 hole at (15, 15) countersunk 90° to Ø10; a 10 x 8 window from (30, 12).
fn detailed(p: V3) -> f64 {
    let plate = boxd(p, [0.0, 0.0, 0.0], [50.0, 30.0, 8.0]);
    // Taken by the bevel: where y / 2 + (8 - z) < 1.
    let bevel = (p[1] / 2.0 + (8.0 - p[2]) - 1.0) / 1.25f64.sqrt();
    // Taken by the round: beyond its quarter circle, centred 2 in from the wall and 2 down from the top.
    let round = (2.0 - (p[0] - 48.0).hypot(p[2] - 6.0))
        .max(48.0 - p[0])
        .max(6.0 - p[2]);
    let cuts = bar(p, 15.0, 15.0, 2.5, -1.0, 9.0)
        .min(cone(p, 15.0, 15.0, 2.5, 5.5, 5.5, 8.5))
        .min(boxd(p, [30.0, 12.0, -1.0], [40.0, 20.0, 9.0]))
        .min(bevel)
        .min(round);
    plate.max(-cuts)
}

/// The part turned 23° about a tilted axis and moved, meshed at `cell`, with noise of about `noise` on each vertex.
fn scanned(cell: f64, noise: f64) -> Mesh {
    scanned_part(&part, [60.0, 40.0, 13.0], cell, noise)
}

/// Any part within `size` from the origin, placed and meshed as `scanned` does.
fn scanned_part(part: &dyn Fn(V3) -> f64, size: V3, cell: f64, noise: f64) -> Mesh {
    let axis = {
        let a = [1.0, 2.0, 3.0];
        let l = 14f64.sqrt();
        a.map(|c| c / l)
    };
    let (s, c) = turn_deg().to_radians().sin_cos();
    // Rodrigues: rotate by +23° to place, by -23° to look back into the part's frame.
    let rotate = |p: V3, s: f64| {
        let d = axis[0] * p[0] + axis[1] * p[1] + axis[2] * p[2];
        let x = [
            axis[1] * p[2] - axis[2] * p[1],
            axis[2] * p[0] - axis[0] * p[2],
            axis[0] * p[1] - axis[1] * p[0],
        ];
        [0, 1, 2].map(|k| p[k] * c + x[k] * s + axis[k] * d * (1.0 - c))
    };
    let shift = [12.5, -40.0, 7.0];
    let placed = |p: V3| {
        part(rotate(
            [p[0] - shift[0], p[1] - shift[1], p[2] - shift[2]],
            -s,
        ))
    };
    // The placed part's bounds: its frame's corners turned and moved, with room around.
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for corner in 0..8 {
        let q = [
            if corner & 1 == 0 { -1.0 } else { size[0] + 1.0 },
            if corner & 2 == 0 { -1.0 } else { size[1] + 1.0 },
            if corner & 4 == 0 { -1.0 } else { size[2] + 1.0 },
        ];
        let r = rotate(q, s);
        for k in 0..3 {
            lo[k] = lo[k].min(r[k] + shift[k] - 2.0 * cell);
            hi[k] = hi[k].max(r[k] + shift[k] + 2.0 * cell);
        }
    }
    // Off the grid by a fraction of a cell, as a scanner's samples are: a face lying exactly on grid nodes makes
    // degenerate facets no real scan has.
    let lo = [
        lo[0] + 0.37 * cell,
        lo[1] + 0.61 * cell,
        lo[2] + 0.29 * cell,
    ];
    let mut m = surface_nets(&placed, lo, hi, cell);
    for (i, p) in m.positions.iter_mut().enumerate() {
        for (k, v) in p.iter_mut().enumerate() {
            // A sum of three uniform draws: close to Gaussian, standard deviation `noise`.
            let u: f64 = (0..3)
                .map(|j| hash_unit((i as u64) * 9 + (k as u64) * 3 + j))
                .sum();
            *v += (u * noise) as f32;
        }
    }
    m
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

fn param(d: &Value, name: &str) -> f64 {
    let e = d["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("no parameter {name}"))["expr"]
        .as_str()
        .unwrap()
        .to_string();
    e.trim_end_matches(" mm").parse().unwrap()
}

#[test]
fn a_noisy_turned_scan_rebuilds_to_its_true_sizes() {
    let mesh = scanned(0.5, 0.02);
    assert!(
        mesh.triangles.len() > 20_000,
        "{} triangles",
        mesh.triangles.len()
    );
    let r = design(&mesh, "scan").unwrap();
    let report = &r.report;
    let d = &r.design;
    // Nothing a person would need to model by hand: only the softened edges are summarised.
    let dropped: Vec<&Value> = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["severity"] == "dropped")
        .collect();
    assert!(dropped.is_empty(), "{dropped:?}");
    let mut sizes = [param(d, "length"), param(d, "width")];
    sizes.sort_by(f64::total_cmp);
    assert!(
        (sizes[0] - 40.0).abs() < 0.05 && (sizes[1] - 60.0).abs() < 0.05,
        "{sizes:?}"
    );
    assert!((param(d, "thickness") - 8.0).abs() < 0.05);

    let holes = report["details"]["holes"].as_array().unwrap();
    assert_eq!(holes.len(), 2, "{holes:?}");
    let through = holes
        .iter()
        .find(|h| h["kind"] == "through")
        .expect("the through hole");
    assert!((num(&through["diameter"]) - 6.0).abs() < 0.05, "{through}");
    let blind = holes
        .iter()
        .find(|h| h["kind"] == "blind from above")
        .expect("the blind hole");
    assert!((num(&blind["diameter"]) - 4.0).abs() < 0.05, "{blind}");
    assert!(
        (num(&blind["from"]) - 3.0).abs() < 0.05,
        "its floor: {blind}"
    );

    let boss = &report["details"]["bosses"][0];
    assert!((num(&boss["diameter"]) - 10.0).abs() < 0.05, "{boss}");
    assert!((num(&boss["standing_height"]) - 5.0).abs() < 0.05, "{boss}");

    let pocket = &report["details"]["pockets"][0];
    let mut ps = [num(&pocket["length"]), num(&pocket["width"])];
    ps.sort_by(f64::total_cmp);
    assert!(
        (ps[0] - 10.0).abs() < 0.1 && (ps[1] - 16.0).abs() < 0.1,
        "{pocket}"
    );
    assert!(
        (num(&pocket["height"]) - 5.0).abs() < 0.05,
        "its floor: {pocket}"
    );

    // The rebuilt part sits on the scan to within its noise.
    let check = &report["details"]["check"];
    assert!(num(&check["p95_mm"]) < 0.1, "{check}");
}

#[test]
fn a_noisy_scan_keeps_its_bevel_round_countersink_and_window() {
    let mesh = scanned_part(&detailed, [50.0, 30.0, 8.0], 0.4, 0.02);
    let r = design(&mesh, "detailed").unwrap();
    let report = &r.report;
    let dropped: Vec<&Value> = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["severity"] == "dropped")
        .collect();
    assert!(dropped.is_empty(), "{dropped:?}");
    let det = &report["details"];
    let chamfers = det["chamfers"].as_array().unwrap();
    assert_eq!(chamfers.len(), 1, "{chamfers:?}");
    assert!(
        (num(&chamfers[0]["width"]) - 2.0).abs() < 0.1,
        "{}",
        chamfers[0]
    );
    assert!(
        (num(&chamfers[0]["height"]) - 1.0).abs() < 0.1,
        "{}",
        chamfers[0]
    );
    let fillets = det["fillets"].as_array().unwrap();
    assert_eq!(fillets.len(), 1, "{fillets:?}");
    assert!(
        (num(&fillets[0]["radius"]) - 2.0).abs() < 0.1,
        "{}",
        fillets[0]
    );
    let hole = &det["holes"][0];
    assert!((num(&hole["diameter"]) - 5.0).abs() < 0.05, "{hole}");
    let csk = &hole["countersink"];
    assert!((num(&csk["diameter"]) - 10.0).abs() < 0.1, "{hole}");
    assert!((num(&csk["depth"]) - 2.5).abs() < 0.1, "{hole}");
    let cutouts = det["cutouts"].as_array().unwrap();
    assert_eq!(cutouts.len(), 1, "{cutouts:?}");
    let pts: Vec<[f64; 2]> = cutouts[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| [num(&p[0]), num(&p[1])])
        .collect();
    let span = |k: usize| {
        let (a, b) = pts
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
                (a.min(p[k]), b.max(p[k]))
            });
        b - a
    };
    let mut size = [span(0), span(1)];
    size.sort_by(f64::total_cmp);
    assert!(
        (size[0] - 8.0).abs() < 0.1 && (size[1] - 10.0).abs() < 0.1,
        "{size:?}"
    );
    assert!(num(&det["check"]["p95_mm"]) < 0.1, "{}", det["check"]);
}
