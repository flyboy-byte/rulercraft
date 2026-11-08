// SPDX-License-Identifier: MIT OR Apache-2.0
//! re.features checks: patterns built from known positions, and design-value guesses.
#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]
use rulercraft_mesh::reverse::{Hole, Pattern, PatternOptions, design_guesses, hole_patterns};

fn hole(name: &str, p: [f64; 3], d: f64) -> Hole {
    Hole {
        name: name.into(),
        point: p,
        axis: [0., 0., 1.],
        diameter: d,
    }
}
fn ring(deg: &[f64]) -> Vec<Hole> {
    deg.iter()
        .enumerate()
        .map(|(i, a)| {
            let a = a.to_radians();
            hole(
                &format!("h{i}"),
                [10. + 25. * a.cos(), 20. + 25. * a.sin(), 0.],
                6.0,
            )
        })
        .collect()
}

#[test]
fn a_bolt_circle_is_found_with_its_centre_radius_and_step() {
    let mut holes = ring(&[0., 60., 120., 180., 240., 300.]);
    holes.push(hole("odd", [0., 0., 0.], 8.0)); // different diameter: not part of it
    let p = hole_patterns(&holes, PatternOptions::default());
    assert_eq!(p.len(), 1, "{p:?}");
    let Pattern::Circle {
        members,
        diameter,
        center,
        radius,
        step_deg,
        ..
    } = &p[0]
    else {
        panic!("{p:?}")
    };
    assert_eq!(members.len(), 6);
    assert!(
        (diameter - 6.0).abs() < 1e-9
            && (radius - 25.0).abs() < 1e-6
            && (step_deg - 60.0).abs() < 1e-6
    );
    assert!((center[0] - 10.0).abs() < 1e-6 && (center[1] - 20.0).abs() < 1e-6);
}

#[test]
fn three_holes_of_an_arc_still_make_a_circle_but_irregular_ones_do_not() {
    assert_eq!(
        hole_patterns(&ring(&[0., 60., 120.]), PatternOptions::default()).len(),
        1
    );
    assert_eq!(
        hole_patterns(&ring(&[0., 60., 180.]), PatternOptions::default()).len(),
        1
    ); // 60, 120, 180 gaps
    assert!(hole_patterns(&ring(&[0., 50., 130.]), PatternOptions::default()).is_empty());
}

#[test]
fn a_row_of_holes_is_found_and_uneven_spacing_is_flagged() {
    let u = [1.0 / 5f64.sqrt(), 2.0 / 5f64.sqrt()];
    let at = |s: f64| [s * u[0], s * u[1], 0.0];
    let even: Vec<Hole> = [0., 15., 30., 45., 60.]
        .iter()
        .enumerate()
        .map(|(i, &s)| hole(&format!("r{i}"), at(s), 5.0))
        .collect();
    let p = hole_patterns(&even, PatternOptions::default());
    let Pattern::Line {
        members,
        spacing,
        equal_spacing,
        ..
    } = &p[0]
    else {
        panic!("{p:?}")
    };
    assert_eq!(members.len(), 5);
    assert!((spacing - 15.0).abs() < 1e-9 && *equal_spacing);
    let uneven: Vec<Hole> = [0., 15., 40.]
        .iter()
        .enumerate()
        .map(|(i, &s)| hole(&format!("r{i}"), at(s), 5.0))
        .collect();
    let Pattern::Line { equal_spacing, .. } = &hole_patterns(&uneven, PatternOptions::default())[0]
    else {
        panic!()
    };
    assert!(!equal_spacing);
}

#[test]
fn turning_the_part_changes_no_pattern_number() {
    let base = hole_patterns(&ring(&[0., 90., 180., 270.]), PatternOptions::default());
    // Turn about x by 40 degrees: positions and axes move together.
    let (s, c) = 40f64.to_radians().sin_cos();
    let turn = |v: [f64; 3]| [v[0], v[1] * c - v[2] * s, v[1] * s + v[2] * c];
    let turned: Vec<Hole> = ring(&[0., 90., 180., 270.])
        .into_iter()
        .map(|h| Hole {
            point: turn(h.point),
            axis: turn(h.axis),
            ..h
        })
        .collect();
    let after = hole_patterns(&turned, PatternOptions::default());
    let (
        Pattern::Circle {
            radius: r1,
            step_deg: s1,
            ..
        },
        Pattern::Circle {
            radius: r2,
            step_deg: s2,
            ..
        },
    ) = (&base[0], &after[0])
    else {
        panic!()
    };
    assert!((r1 - r2).abs() < 1e-9 && (s1 - s2).abs() < 1e-6);
}

#[test]
fn small_groups_and_hostile_holes_give_nothing_and_do_not_panic() {
    assert!(hole_patterns(&[], PatternOptions::default()).is_empty());
    assert!(hole_patterns(&ring(&[0., 90.]), PatternOptions::default()).is_empty());
    let mut bad = ring(&[0., 90., 180.]);
    bad[0].axis = [0.; 3];
    bad[1].diameter = f64::NAN;
    assert!(hole_patterns(&bad, PatternOptions::default()).is_empty());
    // Same position three times: no row, no circle.
    let same: Vec<Hole> = (0..3)
        .map(|i| hole(&format!("s{i}"), [1., 1., 0.], 4.0))
        .collect();
    assert!(hole_patterns(&same, PatternOptions::default()).is_empty());
}

#[test]
fn measured_values_are_matched_to_round_numbers_with_their_error() {
    let g = design_guesses(5.98, 0.05);
    assert!(
        g.iter().any(|x| x.basis == "whole mm"
            && (x.value - 6.0).abs() < 1e-12
            && (x.error + 0.02).abs() < 1e-9),
        "{g:?}"
    );
    assert!(g.iter().any(|x| x.basis == "tenth mm"));
    let i = design_guesses(6.35, 0.02);
    assert!(
        i.iter()
            .any(|x| x.basis == "eighth inch" && (x.value - 6.35).abs() < 1e-9),
        "{i:?}"
    );
    assert!(
        !i.iter()
            .any(|x| x.basis == "whole mm" || x.basis == "tenth mm")
    );
    assert!(
        design_guesses(f64::NAN, 0.1).is_empty()
            && design_guesses(5.0, 0.0).is_empty()
            && design_guesses(0.01, 0.05).is_empty()
    );
}

mod symmetry {
    use rulercraft_mesh::Mesh;
    use rulercraft_mesh::reverse::symmetry_planes;

    /// A box face as an n x n grid; `u × v` outward.
    fn face(m: &mut Mesh, o: [f64; 3], u: [f64; 3], v: [f64; 3], n: u32) {
        let base = m.positions.len() as u32;
        for i in 0..=n {
            for j in 0..=n {
                let (s, t) = (f64::from(i) / f64::from(n), f64::from(j) / f64::from(n));
                m.positions
                    .push([0, 1, 2].map(|k| (o[k] + s * u[k] + t * v[k]) as f32));
            }
        }
        for i in 0..n {
            for j in 0..n {
                let a = base + i * (n + 1) + j;
                m.triangles.push([a, a + n + 1, a + n + 2]);
                m.triangles.push([a, a + n + 2, a + 1]);
            }
        }
    }
    fn boxy(m: &mut Mesh, lo: [f64; 3], s: [f64; 3], n: u32) {
        let (x, y, z) = ([s[0], 0., 0.], [0., s[1], 0.], [0., 0., s[2]]);
        let hi = [lo[0] + s[0], lo[1] + s[1], lo[2] + s[2]];
        face(m, lo, y, x, n);
        face(m, [lo[0], lo[1], hi[2]], x, y, n);
        face(m, lo, z, y, n);
        face(m, [hi[0], lo[1], lo[2]], y, z, n);
        face(m, lo, x, z, n);
        face(m, [lo[0], hi[1], lo[2]], z, x, n);
    }

    /// A 40 x 20 x 10 plate with a 10 mm cube on top at one end, centred across y.
    fn part() -> Mesh {
        let mut m = Mesh::default();
        boxy(&mut m, [0., 0., 0.], [40., 20., 10.], 8);
        boxy(&mut m, [0., 5., 10.], [10., 10., 10.], 4);
        m
    }

    #[test]
    fn the_mirror_plane_across_the_width_is_found_and_the_others_are_not() {
        let r = symmetry_planes(&part(), 0.05).unwrap();
        assert_eq!(r.len(), 3);
        let sym: Vec<_> = r.iter().filter(|s| s.symmetric).collect();
        assert_eq!(sym.len(), 1, "{r:?}");
        assert!(sym[0].normal[1].abs() > 0.999, "{:?}", sym[0]);
        assert!((sym[0].center[1] - 10.0).abs() < 1e-5);
        assert!(
            r.iter().filter(|s| !s.symmetric).all(|s| s.max > 1.0),
            "{r:?}"
        );
    }

    #[test]
    fn moving_the_part_leaves_the_verdict_unchanged() {
        let mut m = part();
        let (s, c) = 0.6f64.sin_cos();
        for p in &mut m.positions {
            let q = p.map(f64::from);
            *p = [
                (q[0] * c - q[1] * s + 100.0) as f32,
                (q[0] * s + q[1] * c - 50.0) as f32,
                q[2] as f32,
            ];
        }
        let r = symmetry_planes(&m, 0.05).unwrap();
        assert_eq!(r.iter().filter(|s| s.symmetric).count(), 1, "{r:?}");
    }

    #[test]
    fn bad_input_is_refused() {
        assert!(symmetry_planes(&part(), 0.0).is_err());
        assert!(symmetry_planes(&part(), f64::NAN).is_err());
        assert!(symmetry_planes(&Mesh::default(), 0.1).is_err());
    }
}

mod grid {
    use super::*;

    fn lattice(cols: u32, rows: u32, da: f64, db: f64, skip: &[(u32, u32)]) -> Vec<Hole> {
        let mut v = vec![];
        for j in 0..rows {
            for i in 0..cols {
                if skip.contains(&(i, j)) {
                    continue;
                }
                v.push(hole(
                    &format!("g{i}_{j}"),
                    [5.0 + f64::from(i) * da, 7.0 + f64::from(j) * db, 0.0],
                    4.0,
                ));
            }
        }
        v
    }

    #[test]
    fn a_rectangular_grid_gives_its_rows_columns_and_spacings() {
        let p = hole_patterns(&lattice(4, 3, 10.0, 15.0, &[]), PatternOptions::default());
        let Pattern::Grid {
            members,
            columns,
            rows,
            spacing_a,
            spacing_b,
            angle_deg,
            full,
            ..
        } = &p[0]
        else {
            panic!("{p:?}")
        };
        assert_eq!((members.len(), *columns, *rows), (12, 4, 3));
        assert!(
            (spacing_a - 10.0).abs() < 1e-9
                && (spacing_b - 15.0).abs() < 1e-9
                && (angle_deg - 90.0).abs() < 1e-6
        );
        assert!(full);
        assert_eq!(members[0], "g0_0");
    }

    #[test]
    fn a_grid_with_a_missing_hole_is_not_full_and_a_scatter_is_not_a_grid() {
        let p = hole_patterns(
            &lattice(4, 3, 10.0, 15.0, &[(2, 1)]),
            PatternOptions::default(),
        );
        let Pattern::Grid { full, members, .. } = &p[0] else {
            panic!("{p:?}")
        };
        assert!(!full && members.len() == 11);
        let mut scatter = lattice(3, 3, 10.0, 15.0, &[]);
        scatter[4].point = [13.7, 20.9, 0.0]; // pulled off its lattice position
        assert!(hole_patterns(&scatter, PatternOptions::default()).is_empty());
        assert!(
            hole_patterns(&lattice(2, 1, 10.0, 15.0, &[]), PatternOptions::default()).is_empty()
        );
    }

    #[test]
    fn two_holes_on_one_lattice_position_are_not_a_grid() {
        let mut g = lattice(3, 3, 10.0, 15.0, &[]);
        let mut twin = g[4].clone();
        twin.name = "twin".into();
        twin.point[0] += 0.05;
        g.push(twin);
        assert!(hole_patterns(&g, PatternOptions::default()).is_empty());
    }

    #[test]
    fn a_turned_grid_reports_the_same_numbers() {
        let (s, c) = 0.7f64.sin_cos();
        let tilted: Vec<Hole> = lattice(3, 3, 12.0, 8.0, &[])
            .into_iter()
            .map(|h| Hole {
                point: [
                    h.point[0] * c - h.point[1] * s,
                    h.point[0] * s + h.point[1] * c,
                    0.0,
                ],
                ..h
            })
            .collect();
        let p = hole_patterns(&tilted, PatternOptions::default());
        let Pattern::Grid {
            columns,
            rows,
            spacing_a,
            spacing_b,
            full,
            ..
        } = &p[0]
        else {
            panic!("{p:?}")
        };
        assert!((*columns, *rows) == (3, 3) || (*columns, *rows) == (3, 3));
        assert!(
            *full
                && (spacing_a.min(*spacing_b) - 8.0).abs() < 1e-9
                && (spacing_a.max(*spacing_b) - 12.0).abs() < 1e-9
        );
    }
}

#[test]
fn distant_hole_does_not_erase_nearby_line_offsets() {
    // Three close points form a right triangle; no line can put all within 0.1 mm.
    // The fourth point is so far away that subtracting it from each close point
    // rounds away their distinct coordinates in f64.
    for far in [1e20, 1e30] {
        let base = vec![
            hole("a", [0.0, 0.0, 0.0], 5.0),
            hole("b", [1.0, 0.0, 0.0], 5.0),
            hole("c", [0.0, 1.0, 0.0], 5.0),
            hole("far", [far, -far, 0.0], 5.0),
        ];
        for shift in 0..base.len() {
            let mut holes = base.clone();
            holes.rotate_left(shift);
            let patterns = hole_patterns(&holes, PatternOptions::default());
            assert!(
                patterns.iter().all(|p| !matches!(p, Pattern::Line { .. })),
                "far={far}, shift={shift}: {patterns:?}"
            );
        }
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn sparse_grid_larger_than_address_space_is_not_full() {
    let points = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1e10, 1e10, 0.0],
    ];
    let holes: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, p)| hole(&i.to_string(), *p, 5.0))
        .collect();
    let patterns = hole_patterns(&holes, PatternOptions::default());
    let [
        Pattern::Grid {
            columns,
            rows,
            full,
            ..
        },
    ] = patterns.as_slice()
    else {
        panic!("{patterns:?}")
    };
    assert_eq!((*columns, *rows), (10_000_000_001, 10_000_000_001));
    assert!(!full);
}

#[test]
fn distant_holes_on_both_sides_do_not_hide_middle_offsets() {
    let points = [
        [-1e20, 1e20, 0.0],
        [1e20, -1e20, 0.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let holes: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, p)| hole(&i.to_string(), *p, 5.0))
        .collect();
    let patterns = hole_patterns(&holes, PatternOptions::default());
    assert!(
        patterns.iter().all(|p| !matches!(p, Pattern::Line { .. })),
        "{patterns:?}"
    );
}

#[test]
fn distant_endpoint_does_not_shuffle_nearby_row_members() {
    let holes = vec![
        hole("zero", [0.0, 0.0, 0.0], 5.0),
        hole("two", [2.0, 0.0, 0.0], 5.0),
        hole("one", [1.0, 0.0, 0.0], 5.0),
        hole("far", [1e20, 0.0, 0.0], 5.0),
    ];
    for pattern in hole_patterns(&holes, PatternOptions::default()) {
        if let Pattern::Line { members, .. } = pattern {
            let coords: Vec<_> = members
                .iter()
                .map(|name| holes.iter().find(|h| &h.name == name).unwrap().point[0])
                .collect();
            assert!(
                coords.windows(2).all(|w| w[0] <= w[1]) || coords.windows(2).all(|w| w[0] >= w[1]),
                "{members:?}"
            );
        }
    }
}

#[test]
fn distant_endpoint_keeps_a_resolvable_line() {
    let points = [2.0, 1.0, 0.0, 1e20];
    let holes: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, &x)| hole(&i.to_string(), [x, 0.0, 0.0], 5.0))
        .collect();
    let patterns = hole_patterns(&holes, PatternOptions::default());
    let [Pattern::Line { worst_offset, .. }] = patterns.as_slice() else {
        panic!("{patterns:?}")
    };
    assert_eq!(*worst_offset, 0.0);
}

#[test]
fn grid_with_unrepresentable_span_is_not_reported() {
    let points = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [9e18, 9e18, 0.0],
        [-9e18, -9e18, 0.0],
    ];
    let holes: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, p)| hole(&i.to_string(), *p, 5.0))
        .collect();
    let patterns = hole_patterns(&holes, PatternOptions::default());
    assert!(
        patterns.iter().all(|p| !matches!(p, Pattern::Grid { .. })),
        "{patterns:?}"
    );
}
