// SPDX-License-Identifier: MIT OR Apache-2.0
//! Drives the real stdio process with known-answer meshes and JSON-RPC.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};
struct Client {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
    id: u64,
}
impl Client {
    fn new() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rulercraft-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    break;
                };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut c = Self {
            child,
            input,
            lines,
            id: 0,
        };
        let v=c.request("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"known-answer-check","version":"1"}}));
        assert!(v["result"]["capabilities"]["tools"].is_object(), "{v}");
        assert!(
            v["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains("mesh.open")
        );
        writeln!(
            c.input,
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        c.input.flush().unwrap();
        c
    }
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        writeln!(
            self.input,
            "{}",
            json!({"jsonrpc":"2.0","id":self.id,"method":method,"params":params})
        )
        .unwrap();
        self.input.flush().unwrap();
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(5))
                .expect("stdio response within five seconds");
            let v: Value = serde_json::from_str(&line).expect("stdout contains only JSON-RPC");
            if v.get("id").is_some() {
                assert_eq!(v["id"], self.id);
                return v;
            }
        }
    }
    fn call(&mut self, name: &str, args: Value) -> Value {
        let v = self.request("tools/call", json!({"name":name,"arguments":args}));
        assert!(v.get("error").is_none(), "{v}");
        v["result"].clone()
    }
    fn data(&mut self, name: &str, args: Value) -> Value {
        let r = self.call(name, args);
        assert_ne!(r["isError"], true, "{r}");
        serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap()
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn ply(p: &[[f64; 3]], faces: &[[u32; 3]]) -> String {
    let mut s = format!(
        "ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n",
        p.len(),
        faces.len()
    );
    for v in p {
        s += &format!("{} {} {}\n", v[0], v[1], v[2]);
    }
    for f in faces {
        s += &format!("3 {} {} {}\n", f[0], f[1], f[2]);
    }
    s
}
fn cube() -> String {
    let p: Vec<_> = (0..8)
        .map(|i| {
            [
                (i & 1) as f64 * 20.,
                ((i >> 1) & 1) as f64 * 20.,
                ((i >> 2) & 1) as f64 * 20.,
            ]
        })
        .collect();
    let mut t = Vec::new();
    for q in [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        t.extend([[q[0], q[1], q[2]], [q[0], q[2], q[3]]]);
    }
    ply(&p, &t)
}
fn tube(sides: usize) -> String {
    let mut positions = Vec::new();
    for row in 0..=4 {
        let y = 15.0 * row as f64 / 4.0;
        for i in 0..sides {
            let angle = std::f64::consts::TAU * i as f64 / sides as f64;
            positions.push([3.0 + 6.0 * angle.cos(), y, -2.0 + 6.0 * angle.sin()]);
        }
    }
    let mut faces = Vec::new();
    for row in 0..4 {
        for i in 0..sides {
            let a = (row * sides + i) as u32;
            let b = (row * sides + (i + 1) % sides) as u32;
            let c = b + sides as u32;
            let d = a + sides as u32;
            faces.extend([[a, b, c], [a, c, d]]);
        }
    }
    ply(&positions, &faces)
}

#[test]
fn stdio_tools_schemas_geometry_and_failure_state() {
    let dir = std::env::temp_dir().join(format!("rulercraft-mcp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("known.ply");
    std::fs::write(&path, cube()).unwrap();
    let mut c = Client::new();
    let tools = c.request("tools/list", json!({}));
    let tools = tools["result"]["tools"].as_array().unwrap();
    for name in [
        "mesh.open",
        "mesh.info",
        "select",
        "fit",
        "measure",
        "datum",
        "segment",
        "history.info",
        "history.undo",
        "history.redo",
        "guide",
    ] {
        let t = tools.iter().find(|t| t["name"] == name).unwrap();
        assert!(t["inputSchema"].is_object());
    }
    assert_eq!(c.call("mesh.info", json!({}))["isError"], true);
    let m = c.data("mesh.open", json!({"path":path}));
    assert_eq!(m["vertices"], 8);
    assert_eq!(m["triangles"], 12);
    assert_eq!(m["bounds"]["max"], json!([20., 20., 20.]));
    let sel = c.data("select", json!({"limit":3}));
    assert_eq!(sel["total"], 8);
    assert_eq!(sel["indices"], json!([0, 1, 2]));
    assert_eq!(sel["truncated"], true);
    let sel = c.data("select", json!({"offset":7,"limit":3}));
    assert_eq!(sel["indices"], json!([7]));
    assert_eq!(sel["truncated"], false);
    let top = json!({"in_box":[[-1,-1,19],[21,21,21]]});
    let bottom = json!({"in_box":[[-1,-1,-1],[21,21,1]]});
    let f = c.data("fit", json!({"kind":"plane","selection":top}));
    assert_eq!(f["selected"], 4);
    assert_eq!(f["fit"]["center"][2], 20.);
    assert_eq!(f["fit"]["normal"], json!([0., 0., 1.]));
    let m=c.data("measure",json!({"first":{"kind":"plane","selection":bottom},"second":{"kind":"plane","selection":top}}));
    assert_eq!(m["relation"]["distance"], 20.);
    assert_eq!(m["relation"]["angle_deg"], 0.);
    let primary = json!({"kind":"plane","selection":{"in_box":[[-1,-1,19],[21,21,21]]}});
    let secondary = json!({"kind":"plane","selection":{"in_box":[[19,-1,-1],[21,21,21]]}});
    let tertiary = json!({"kind":"plane","selection":{"in_box":[[-1,19,-1],[21,21,21]]}});
    let datum = c.data(
        "datum",
        json!({"primary":primary,"secondary":secondary,"tertiary":tertiary}),
    );
    let rotation = datum["transform"]["rotation"].as_array().unwrap();
    for (i, row) in rotation.iter().enumerate() {
        for (j, value) in row.as_array().unwrap().iter().enumerate() {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!((value.as_f64().unwrap() - expected).abs() < 1e-12);
        }
        assert!((datum["transform"]["translation"][i].as_f64().unwrap() + 20.0).abs() < 1e-12);
    }
    let parallel = c.call(
        "datum",
        json!({"primary":primary,"secondary":primary,"tertiary":tertiary}),
    );
    assert_eq!(parallel["isError"], true);
    let wrong_kind = c.call(
        "datum",
        json!({"primary":{"kind":"sphere"},"secondary":secondary,"tertiary":tertiary}),
    );
    assert_eq!(wrong_kind["isError"], true);
    assert_eq!(c.data("mesh.info", json!({}))["vertices"], 8);
    let segmented = c.data("segment", json!({}));
    assert_eq!(segmented["kind"], "cylinders");
    assert_eq!(segmented["regions"].as_array().unwrap().len(), 0);
    assert_eq!(segmented["unassigned"], 12);
    let segmented = c.data("segment", json!({"kind":"planes"}));
    assert_eq!(segmented["regions"].as_array().unwrap().len(), 6);
    let segmented = c.data("segment", json!({"kind":"strips"}));
    assert_eq!(segmented["regions"].as_array().unwrap().len(), 0);
    assert_eq!(
        c.call("segment", json!({"kind":"unknown"}))["isError"],
        true
    );
    assert_eq!(
        c.call("mesh.open", json!({"path":dir.join("missing.stl")}))["isError"],
        true
    );
    assert_eq!(c.data("mesh.info", json!({}))["vertices"], 8);
    assert_eq!(
        c.call("mesh.open", json!({"path":path,"max_bytes":1}))["isError"],
        true
    );
    assert_eq!(c.data("mesh.info", json!({}))["triangles"], 12);
    assert_eq!(
        c.call(
            "fit",
            json!({"kind":"plane","selection":{"near":{"center":[99,99,99],"radius":1}}})
        )["isError"],
        true
    );
    assert!(
        c.request("tools/call", json!({"name":"unknown","arguments":{}}))
            .get("error")
            .is_some()
    );
    let p = [
        [15., -3., 2.],
        [-5., -3., 2.],
        [5., 7., 2.],
        [5., -13., 2.],
        [5., -3., 12.],
        [5., -3., -8.],
    ];
    std::fs::write(
        &path,
        ply(
            &p,
            &[
                [0, 2, 4],
                [2, 1, 4],
                [1, 3, 4],
                [3, 0, 4],
                [2, 0, 5],
                [1, 2, 5],
                [3, 1, 5],
                [0, 3, 5],
            ],
        ),
    )
    .unwrap();
    c.data("mesh.open", json!({"path":path}));
    let s = c.data("fit", json!({"kind":"sphere","sigma":0}));
    assert!((s["fit"]["radius"].as_f64().unwrap() - 10.).abs() < 1e-5);
    let mut p = Vec::new();
    let mut t = Vec::new();
    for z in [0., 20., 40.] {
        for i in 0..32 {
            let a = i as f64 * std::f64::consts::TAU / 32.;
            p.push([10. * a.cos(), 10. * a.sin(), z]);
        }
    }
    for ring in 0..2 {
        for i in 0..32 {
            let a = ring * 32 + i;
            let b = ring * 32 + (i + 1) % 32;
            t.extend([[a, b, b + 32], [a, b + 32, a + 32]]);
        }
    }
    std::fs::write(&path, ply(&p, &t)).unwrap();
    c.data("mesh.open", json!({"path":path}));
    let s = c.data("fit", json!({"kind":"cylinder","sigma":0}));
    assert!((s["fit"]["radius"].as_f64().unwrap() - 10.).abs() < 1e-5);
    assert!((s["fit"]["length"].as_f64().unwrap() - 40.).abs() < 1e-5);

    let fine_tube = dir.join("fine-tube.ply");
    std::fs::write(&fine_tube, tube(32)).unwrap();
    c.data("mesh.open", json!({"path":fine_tube}));
    let segmented = c.data("segment", json!({}));
    assert_eq!(segmented["regions"].as_array().unwrap().len(), 1);
    assert!((segmented["regions"][0]["fit"]["radius"].as_f64().unwrap() - 6.0).abs() < 1e-4);
    let coarse_tube = dir.join("coarse-tube.ply");
    std::fs::write(&coarse_tube, tube(12)).unwrap();
    c.data("mesh.open", json!({"path":coarse_tube}));
    let segmented = c.data("segment", json!({"kind":"strips"}));
    assert_eq!(segmented["regions"].as_array().unwrap().len(), 1);
    assert_eq!(segmented["regions"][0]["strip_count"], 12);
    let a = dir.join("history-a.ply");
    let b = dir.join("history-b.ply");
    let bytes_a = cube().into_bytes();
    let bytes_b = bytes_a.clone();
    std::fs::write(&a, &bytes_a).unwrap();
    std::fs::write(&b, &bytes_b).unwrap();
    c = Client::new();
    c.data("mesh.open", json!({"path":a}));
    c.data("mesh.open", json!({"path":b}));
    let history = c.data("history.info", json!({}));
    assert_eq!(history["position"], 2);
    assert_eq!(history["steps"], 2);
    assert_eq!(history["can_undo"], true);
    assert_eq!(history["can_redo"], false);

    // A changed source refuses a replay without moving the current mesh or cursor.
    std::fs::write(&a, b"changed source").unwrap();
    assert_eq!(c.call("history.undo", json!({}))["isError"], true);
    assert_eq!(
        c.data("mesh.info", json!({}))["path"],
        b.to_string_lossy().as_ref()
    );
    assert_eq!(c.data("history.info", json!({}))["position"], 2);
    std::fs::write(&a, &bytes_a).unwrap();

    let undone = c.data("history.undo", json!({}));
    assert_eq!(undone["mesh"]["path"], a.to_string_lossy().as_ref());
    assert_eq!(c.data("history.info", json!({}))["position"], 1);
    let undone = c.data("history.undo", json!({}));
    assert_eq!(undone["open"], false);
    assert_eq!(c.call("mesh.info", json!({}))["isError"], true);
    let redone = c.data("history.redo", json!({}));
    assert_eq!(redone["mesh"]["path"], a.to_string_lossy().as_ref());
    let redone = c.data("history.redo", json!({}));
    assert_eq!(redone["mesh"]["path"], b.to_string_lossy().as_ref());

    c.data("history.undo", json!({}));
    c.data("mesh.open", json!({"path":a}));
    let history = c.data("history.info", json!({}));
    assert_eq!(history["can_redo"], false);
    assert_eq!(history["steps"], 2);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compare_measures_distance_to_a_reference_file_and_reports_patches() {
    let dir = std::env::temp_dir().join(format!("rulercraft-mcp-cmp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (reference, scan) = (dir.join("reference.ply"), dir.join("scan.ply"));
    std::fs::write(&reference, cube()).unwrap();
    std::fs::write(&scan, cube().replace("20", "21")).unwrap(); // 21 mm cube from the same corner
    let mut c = Client::new();
    assert_eq!(
        c.call("compare", json!({"reference":reference}))["isError"],
        true
    );
    c.data("mesh.open", json!({"path":scan}));
    let r = c.data("compare", json!({"reference":reference,"tolerance":0.5}));
    assert_eq!(r["stats"]["count"], 8);
    // The far corner is sqrt(3) mm off the reference corner; the origin corner is on it.
    assert!(
        (r["stats"]["max_outside"].as_f64().unwrap() - 3f64.sqrt()).abs() < 1e-5,
        "{r}"
    );
    assert_eq!(r["stats"]["max_inside"], 0.0);
    assert_eq!(r["stats"]["beyond_tolerance"], 7);
    assert_eq!(r["patches"]["total"], 1);
    assert_eq!(r["patches"]["list"][0]["vertices"], 7);
    assert_eq!(r["patches"]["list"][0]["outside"], true);
    assert!(r["icp"].is_null());
    // Bad tolerance and a missing reference are plain errors, and the open mesh survives.
    assert_eq!(
        c.call("compare", json!({"reference":reference,"tolerance":-1}))["isError"],
        true
    );
    assert_eq!(
        c.call("compare", json!({"reference":dir.join("none.ply")}))["isError"],
        true
    );
    assert_eq!(c.data("mesh.info", json!({}))["vertices"], 8);
    // Aligning identical meshes leaves them where they are.
    c.data("mesh.open", json!({"path":reference}));
    let a = c.data("compare", json!({"reference":reference,"align":true}));
    assert!(a["icp"]["rms_after"].as_f64().unwrap() < 1e-6, "{a}");
}

#[test]
fn robust_plane_fit_ignores_far_outliers_the_plain_fit_follows() {
    // A flat 4 x 4 grid at z = 0 (spacing 5) plus three vertices 30 mm above it.
    let mut p: Vec<[f64; 3]> = (0..16)
        .map(|i| [f64::from(i % 4) * 5.0, f64::from(i / 4) * 5.0, 0.0])
        .collect();
    p.extend([[1.0, 1.0, 30.0], [11.0, 6.0, 30.0], [6.0, 11.0, 30.0]]);
    let mut t: Vec<[u32; 3]> = vec![];
    for j in 0..3u32 {
        for i in 0..3u32 {
            let a = j * 4 + i;
            t.extend([[a, a + 1, a + 5], [a, a + 5, a + 4]]);
        }
    }
    t.extend([[16, 17, 18]]);
    let dir = std::env::temp_dir().join(format!("rulercraft-mcp-rob-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("noisy.ply");
    std::fs::write(&path, ply(&p, &t)).unwrap();
    let mut c = Client::new();
    c.data("mesh.open", json!({"path":path}));
    let plain = c.data("fit", json!({"kind":"plane","sigma":0}));
    assert!(plain["fit"]["center"][2].as_f64().unwrap() > 1.0, "{plain}");
    for robust in [false, true] {
        let invalid = c.call(
            "fit",
            json!({"kind":"plane","robust":robust,"known_diameter":10.0}),
        );
        assert_eq!(invalid["isError"], true, "{invalid}");
        assert!(
            invalid["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("known diameter")
        );
    }
    let robust = c.data("fit", json!({"kind":"plane","robust":true}));
    assert_eq!(robust["inliers"], 16, "{robust}");
    assert!(robust["fit"]["center"][2].as_f64().unwrap().abs() < 1e-9);
    assert_eq!(robust["fit"]["normal"], json!([0., 0., 1.]));
    let bad = c.call("fit", json!({"kind":"sphere","robust":true}));
    assert_eq!(bad["isError"], true);
    let bad = c.call(
        "fit",
        json!({"kind":"plane","robust":true,"robust_threshold":0}),
    );
    assert_eq!(bad["isError"], true);
}

#[test]
fn reverse_returns_a_recipe_guesses_and_symmetry_for_known_shapes() {
    let dir = std::env::temp_dir().join(format!("rulercraft-mcp-rev-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (cube_path, tube_path) = (dir.join("cube.ply"), dir.join("tube.ply"));
    std::fs::write(&cube_path, cube()).unwrap();
    std::fs::write(&tube_path, tube(32)).unwrap();
    let mut c = Client::new();
    assert_eq!(c.call("reverse", json!({}))["isError"], true); // nothing open yet
    c.data("mesh.open", json!({"path":cube_path}));
    let r = c.data("reverse", json!({}));
    assert_eq!(r["planes_found"], 6, "{r}");
    let steps = r["recipe"]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["step"], "base");
    assert_eq!(steps[0]["size"], json!([20.0, 20.0]));
    assert_eq!(
        steps
            .iter()
            .filter(|s| s["step"] == "parallel_face")
            .count(),
        1
    );
    assert_eq!(steps.iter().filter(|s| s["step"] == "unplaced").count(), 4);
    assert_eq!(r["symmetry"].as_array().unwrap().len(), 3);
    // Every cube vertex lies on three of the six faces the recipe was built from.
    assert_eq!(r["recipe_check"]["explained_fraction"], 1.0, "{r}");
    assert_eq!(r["recipe_check"]["unexplained_count"], 0);
    c.data("mesh.open", json!({"path":tube_path}));
    let r = c.data("reverse", json!({}));
    assert_eq!(r["cylinders_found"], 1, "{r}");
    let g = &r["design_value_guesses"][0];
    assert!(
        (g["measured_diameter"].as_f64().unwrap() - 12.0).abs() < 1e-3,
        "{g}"
    );
    assert!(
        g["guesses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["basis"] == "whole mm" && x["value"] == 12.0),
        "{g}"
    );
}

/// A 2 x 4 x 6 box with a vertex at each face centre, so its normal rays have exact answers.
fn thickness_box() -> String {
    let mut positions = Vec::new();
    for i in 0..8 {
        positions.push([
            (i & 1) as f64 * 2.0,
            ((i >> 1) & 1) as f64 * 4.0,
            ((i >> 2) & 1) as f64 * 6.0,
        ]);
    }
    let quads = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    let mut faces = Vec::new();
    for q in quads {
        let center =
            [0, 1, 2].map(|axis| q.iter().map(|i| positions[*i as usize][axis]).sum::<f64>() / 4.0);
        let id = positions.len() as u32;
        positions.push(center);
        for i in 0..4 {
            faces.push([id, q[i], q[(i + 1) % 4]]);
        }
    }
    ply(&positions, &faces)
}

#[test]
fn thickness_tool_measures_a_known_wall_and_pages_results() {
    let dir = std::env::temp_dir().join(format!("rulercraft-thickness-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("box.ply");
    std::fs::write(&path, thickness_box()).unwrap();
    let mut c = Client::new();
    let selection = json!({"in_box":[[1.0,2.0,6.0],[1.0,2.0,6.0]]});
    assert_eq!(
        c.call("thickness", json!({"selection":selection}))["isError"],
        true
    );
    c.data("mesh.open", json!({"path":path}));
    let v = c.data("thickness", json!({"selection":selection}));
    assert_eq!(v["total"], 1);
    assert_eq!(v["returned"], 1);
    assert!((v["samples"][0]["distance"].as_f64().unwrap() - 6.0).abs() < 1e-10);
    let short = c.data(
        "thickness",
        json!({"selection":selection,"max_distance":1.0}),
    );
    assert!(short["samples"][0]["distance"].is_null());
    let page = c.data("thickness", json!({"offset":0,"limit":2}));
    assert_eq!(page["total"], 14);
    assert_eq!(page["returned"], 2);
    assert_eq!(page["truncated"], true);
    assert_eq!(c.call("thickness", json!({"limit":1025}))["isError"], true);
    assert_eq!(
        c.call("thickness", json!({"max_distance":0.0}))["isError"],
        true
    );
    assert_eq!(c.data("mesh.info", json!({}))["vertices"], 14);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn known_hole_fits_agree_between_mcp_and_library() {
    use rulercraft_mesh::commands::{FitKind, FitRequest, Fitted, Selection, fit};
    use rulercraft_mesh::fit::Clip;
    use rulercraft_mesh::graph::MeshGraph;
    use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/known/hole-block.stl");
    let mesh = read_mesh(
        "hole-block.stl",
        &std::fs::read(&path).unwrap(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    let graph = MeshGraph::build(&mesh);
    let mut client = Client::new();
    client.data("mesh.open", json!({"path":path}));
    for (min, max, expected_diameter) in [
        ([8.4, 13.4, 1.0], [11.6, 16.6, 9.0], 3.0),
        ([27.4, 12.4, 1.0], [32.6, 17.6, 9.0], 5.0),
        ([45.9, 10.9, 1.0], [54.1, 19.1, 9.0], 8.0),
    ] {
        let request = FitRequest {
            kind: FitKind::Cylinder,
            selection: Selection {
                in_box: Some((min, max)),
                ..Default::default()
            },
            clip: Clip::None,
            known_diameter: None,
        };
        let result = fit(&mesh, &graph, &request).unwrap();
        let Fitted::Cylinder(cylinder) = result.fitted else {
            panic!("expected cylinder")
        };
        let mcp = client.data(
            "fit",
            json!({"kind":"cylinder", "selection":{"in_box":[min, max]}, "sigma":0}),
        );
        assert_eq!(mcp["selected"], result.selected);
        let output = &mcp["fit"];
        for (name, value) in [
            ("radius", cylinder.radius),
            ("diameter", cylinder.diameter),
            ("length", cylinder.length),
            ("sigma", cylinder.sigma),
        ] {
            assert_eq!(output[name].as_f64().unwrap(), value, "{name}");
        }
        assert_eq!(output["axis"], json!(cylinder.axis));
        assert_eq!(output["axis_point"], json!(cylinder.axis_point));
        assert!((cylinder.diameter - expected_diameter).abs() < 1e-5);
    }
}

#[test]
fn known_top_plane_agrees_between_mcp_and_library() {
    use rulercraft_mesh::commands::{FitKind, FitRequest, Fitted, Selection, fit};
    use rulercraft_mesh::fit::Clip;
    use rulercraft_mesh::graph::MeshGraph;
    use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/known/hole-block.stl");
    let mesh = read_mesh(
        "hole-block.stl",
        &std::fs::read(&path).unwrap(),
        Unit::Mm,
        ImportLimits::default(),
    )
    .unwrap();
    let graph = MeshGraph::build(&mesh);
    let min = [-1.0, -1.0, 9.999];
    let max = [61.0, 31.0, 10.001];
    let request = FitRequest {
        kind: FitKind::Plane,
        selection: Selection {
            in_box: Some((min, max)),
            ..Default::default()
        },
        clip: Clip::None,
        known_diameter: None,
    };
    let result = fit(&mesh, &graph, &request).unwrap();
    let Fitted::Plane(plane) = result.fitted else {
        panic!("expected plane")
    };
    let mut client = Client::new();
    client.data("mesh.open", json!({"path":path}));
    let output = client.data(
        "fit",
        json!({"kind":"plane", "selection":{"in_box":[min,max]}, "sigma":0}),
    );
    assert_eq!(output["selected"], result.selected);
    assert_eq!(output["fit"]["center"], json!(plane.center));
    assert_eq!(output["fit"]["normal"], json!(plane.normal));
    assert_eq!(output["fit"]["sigma"].as_f64().unwrap(), plane.sigma);
    assert!((plane.center[2] - 10.0).abs() < 1e-5);
    assert!(plane.normal[2].abs() > 1.0 - 1e-10);
}
