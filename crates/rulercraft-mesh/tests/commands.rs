// SPDX-License-Identifier: MIT OR Apache-2.0
//! The shared command layer combines selection filters and refuses mismatched options.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::commands::{CommandError, FitKind, FitRequest, Selection, fit, select};
use rulercraft_mesh::fit::Clip;
use rulercraft_mesh::graph::MeshGraph;

fn grid() -> Mesh {
    let n = 10u32;
    let positions = (0..n * n)
        .map(|i| [(i % n) as f32, (i / n) as f32, 0.0])
        .collect();
    let mut triangles = vec![];
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let a = y * n + x;
            triangles.push([a, a + 1, a + n]);
            triangles.push([a + 1, a + n + 1, a + n]);
        }
    }
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

#[test]
fn filters_combine_and_feed_the_fit() {
    let m = grid();
    let g = MeshGraph::build(&m);
    let s = Selection {
        in_box: Some(([0.0; 3], [4.0, 4.0, 0.0])),
        near: Some(([0.0; 3], 2.0)),
        facing: None,
    };
    assert_eq!(select(&m, &g, &s).unwrap(), vec![0, 1, 2, 10, 11, 20]);
    let up = Selection {
        facing: Some(([0.0, 0.0, 1.0], 0.1)),
        ..Selection::default()
    };
    assert_eq!(select(&m, &g, &up).unwrap().len(), 100);
    let req = FitRequest {
        kind: FitKind::Plane,
        selection: s,
        clip: Clip::Sigma(3),
        known_diameter: None,
    };
    assert_eq!(fit(&m, &g, &req).unwrap().selected, 6);
    let bad = FitRequest {
        known_diameter: Some(3.0),
        ..req
    };
    assert_eq!(
        fit(&m, &g, &bad),
        Err(CommandError::Invalid(
            "a known diameter only applies to a sphere or cylinder fit"
        ))
    );
}
