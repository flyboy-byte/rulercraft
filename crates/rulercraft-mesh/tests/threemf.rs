// SPDX-License-Identifier: MIT OR Apache-2.0
//! Original 3MF packages with fixed geometric answers.
#![allow(clippy::unwrap_used)]
use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
use std::io::{Cursor, Write};
fn package(model: &str, method: zip::CompressionMethod) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(method);
    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(b"<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='r' Type='http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel' Target='/3D/part.model'/></Relationships>").unwrap();
    zip.start_file("3D/part.model", options).unwrap();
    zip.write_all(model.as_bytes()).unwrap();
    zip.finish().unwrap().into_inner()
}
fn cube() -> String {
    let mut out = String::from("<object id='1'><mesh><vertices>");
    for i in 0..8 {
        out += &format!(
            "<vertex x='{}' y='{}' z='{}'/>",
            i & 1,
            (i >> 1) & 1,
            (i >> 2) & 1
        );
    }
    out += "</vertices><triangles>";
    for q in [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
            out += &format!("<triangle v1='{}' v2='{}' v3='{}'/>", t[0], t[1], t[2]);
        }
    }
    out + "</triangles></mesh></object>"
}
fn model(resources: &str, build: &str, unit: &str) -> String {
    format!(
        "<?xml version='1.0' encoding='UTF-8'?><model xmlns='http://schemas.microsoft.com/3dmanufacturing/core/2015/02' unit='{unit}'><resources>{resources}</resources><build>{build}</build></model>"
    )
}
#[test]
fn cube_zip_methods_units_and_nested_transforms() {
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        for (unit, scale) in [
            ("millimeter", 1.0),
            ("centimeter", 10.0),
            ("inch", 25.4),
            ("micron", 0.001),
            ("foot", 304.8),
            ("meter", 1000.0),
        ] {
            let xml = model(&cube(), "<item objectid='1'/>", unit);
            let m = read_mesh(
                "cube.3MF",
                &package(&xml, method),
                Unit::M,
                ImportLimits::default(),
            )
            .unwrap();
            assert_eq!((m.positions.len(), m.triangles.len()), (8, 12));
            assert_eq!(m.bounds().unwrap().min, [0.0; 3]);
            assert_eq!(m.bounds().unwrap().max, [scale; 3]);
        }
    }
    let resources = cube()
        + "<object id='2'><components><component objectid='1' transform='2 0 0 0 3 0 0 0 4 1 2 3'/></components></object>";
    // Child scales/translates, then parent rotates +90 degrees about z and translates x by 10.
    let xml = model(
        &resources,
        "<item objectid='2' transform='0 1 0 -1 0 0 0 0 1 10 0 0'/>",
        "millimeter",
    );
    let m = read_mesh(
        "nested.3mf",
        &package(&xml, zip::CompressionMethod::Deflated),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    assert_eq!(m.bounds().unwrap().min, [5.0, 1.0, 3.0]);
    assert_eq!(m.bounds().unwrap().max, [8.0, 3.0, 7.0]);
}
#[test]
fn clean_refusals_and_limits() {
    let valid = model(&cube(), "<item objectid='1'/>", "millimeter");
    for bad in [
        valid.replace("x='0'", "x='NaN'"),
        valid.replace("v1='0'", "v1='99'"),
        valid.replace("objectid='1'", "objectid='9'"),
        valid.replace("<build>", "<build><broken>"),
        valid.replace("unit='millimeter'", "requiredextensions='x'"),
        valid.replace("<model ", "<!DOCTYPE model><model "),
    ] {
        let e = read_mesh(
            "bad.3mf",
            &package(&bad, zip::CompressionMethod::Stored),
            Unit::Mm,
            ImportLimits::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("offset"), "{e}");
    }
    for limits in [
        ImportLimits {
            max_bytes: 1,
            ..Default::default()
        },
        ImportLimits {
            max_vertices: 7,
            ..Default::default()
        },
        ImportLimits {
            max_triangles: 11,
            ..Default::default()
        },
    ] {
        assert!(
            read_mesh(
                "cube.3mf",
                &package(&valid, zip::CompressionMethod::Stored),
                Unit::Mm,
                limits
            )
            .is_err()
        );
    }
    let cycle = model(
        "<object id='1'><components><component objectid='1'/></components></object>",
        "<item objectid='1'/>",
        "millimeter",
    );
    assert!(
        read_mesh(
            "cycle.3mf",
            &package(&cycle, zip::CompressionMethod::Stored),
            Unit::Mm,
            Default::default()
        )
        .unwrap_err()
        .to_string()
        .contains("cycle")
    );
    assert!(
        read_mesh("bad.3mf", b"not zip", Unit::Mm, Default::default())
            .unwrap_err()
            .to_string()
            .contains("ZIP")
    );
}

#[test]
fn namespace_reflection_repeats_and_degenerates() {
    let resources = cube().replace(
        "</triangles>",
        "<triangle v1='0' v2='0' v3='1'/></triangles>",
    );
    let xml = model(
        &resources,
        "<item objectid='1' transform='-1 0 0 0 1 0 0 0 1 0 0 0'/><item objectid='1' printable='false'/>",
        "millimeter",
    );
    let xml = xml
        .replace('<', "<c:")
        .replace("<c:/", "</c:")
        .replace("<c:?", "<?")
        .replace("xmlns=", "xmlns:c=");
    let m = read_mesh(
        "mirror.3mf",
        &package(&xml, zip::CompressionMethod::Deflated),
        Unit::Mm,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        (m.positions.len(), m.triangles.len(), m.degenerate_dropped),
        (8, 12, 1)
    );
    let volume: f64 = m
        .triangles
        .iter()
        .map(|t| {
            let a = m.positions[t[0] as usize].map(f64::from);
            let b = m.positions[t[1] as usize].map(f64::from);
            let c = m.positions[t[2] as usize].map(f64::from);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum();
    assert!((volume - 1.0).abs() < 1e-12); // integer cube, f64 summation only
    let xml = model(
        &cube(),
        "<item objectid='1'/><item objectid='1'/>",
        "millimeter",
    );
    let bytes = package(&xml, zip::CompressionMethod::Stored);
    let m = read_mesh("repeat.3mf", &bytes, Unit::Mm, Default::default()).unwrap();
    assert_eq!((m.positions.len(), m.triangles.len()), (8, 24));
    assert!(
        read_mesh(
            "repeat.3mf",
            &bytes,
            Unit::Mm,
            ImportLimits {
                max_vertices: 8,
                ..Default::default()
            }
        )
        .unwrap_err()
        .to_string()
        .contains("expanded")
    );
}

#[test]
fn large_inherited_namespace_set_preserves_child_scope() {
    let namespaces: String = (0..2000)
        .map(|i| format!(" xmlns:n{i}='urn:ns{i}'"))
        .collect();
    let mut resources = String::from(
        "<object id='1'><mesh><vertices><p:vertex xmlns:p='urn:other' x='99' y='99' z='99'/>",
    );
    for _ in 0..500 {
        resources.push_str("<p:vertex x='0' y='0' z='0'/><p:vertex x='1' y='0' z='0'/><p:vertex x='0' y='1' z='0'/>");
    }
    resources.push_str(
        "</vertices><triangles><triangle v1='0' v2='1' v3='2'/></triangles></mesh></object>",
    );
    let xml = model(&resources, "<item objectid='1'/>", "millimeter").replace("unit='millimeter'", &format!("unit='millimeter' xmlns:p='http://schemas.microsoft.com/3dmanufacturing/core/2015/02'{namespaces}"));
    let mesh = read_mesh(
        "namespaces.3mf",
        &package(&xml, zip::CompressionMethod::Deflated),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    assert_eq!((mesh.positions.len(), mesh.triangles.len()), (3, 1));
    assert_eq!(mesh.bounds().unwrap().max, [1.0, 1.0, 0.0]);
}
