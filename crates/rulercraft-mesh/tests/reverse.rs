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
