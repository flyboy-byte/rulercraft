// SPDX-License-Identifier: MIT OR Apache-2.0
//! The GUI server over a real socket, with a cube of known size.
#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]
use rulercraft_gui::{State, serve};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

const CUBE: &str = "ply\nformat ascii 1.0\nelement vertex 8\nproperty float x\nproperty float y\nproperty float z\nelement face 12\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n20 0 0\n0 20 0\n20 20 0\n0 0 20\n20 0 20\n0 20 20\n20 20 20\n3 0 2 3\n3 0 3 1\n3 4 5 7\n3 4 7 6\n3 0 1 5\n3 0 5 4\n3 2 6 7\n3 2 7 3\n3 0 4 6\n3 0 6 2\n3 1 3 7\n3 1 7 5\n";

struct Server {
    port: u16,
}
fn start(open_cube: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut st = State::default();
    if open_cube {
        st.open_bytes("cube.ply", CUBE.as_bytes(), rulercraft_mesh::Unit::Mm)
            .unwrap();
    }
    let st = Arc::new(Mutex::new(st));
    std::thread::spawn(move || serve(listener, st));
    Server { port }
}
impl Server {
    fn raw(&self, method: &str, path: &str, host: Option<&str>, body: &[u8]) -> (u16, Vec<u8>) {
        self.raw_origin(method, path, host, None, body)
    }
    fn raw_origin(
        &self,
        method: &str,
        path: &str,
        host: Option<&str>,
        origin: Option<&str>,
        body: &[u8],
    ) -> (u16, Vec<u8>) {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        let host = host.map_or(format!("127.0.0.1:{}", self.port), str::to_string);
        let origin = origin.map_or(String::new(), |o| format!("Origin: {o}\r\n"));
        write!(
            s,
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{origin}Content-Length: {}\r\n\r\n",
            body.len()
        )
        .unwrap();
        s.write_all(body).unwrap();
        let mut out = vec![];
        s.read_to_end(&mut out).unwrap();
        let split = out.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let status = std::str::from_utf8(&out[..split])
            .unwrap()
            .split(' ')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (status, out[split + 4..].to_vec())
    }
    fn json(&self, path: &str, body: &Value) -> (u16, Value) {
        let (s, b) = self.raw("POST", path, None, body.to_string().as_bytes());
        (s, serde_json::from_slice(&b).unwrap())
    }
}

#[test]
fn the_page_info_and_buffers_are_served() {
    let s = start(true);
    let (st, page) = s.raw("GET", "/", None, b"");
    assert_eq!(st, 200);
    assert!(
        String::from_utf8(page)
            .unwrap()
            .contains("<title>RulerCraft")
    );
    let (_, info) = s.raw("GET", "/api/info", None, b"");
    let info: Value = serde_json::from_slice(&info).unwrap();
    assert_eq!(
        (info["vertices"].as_u64(), info["triangles"].as_u64()),
        (Some(8), Some(12))
    );
    assert_eq!(info["bounds"]["max"], json!([20.0, 20.0, 20.0]));
    assert_eq!(info["name"], "cube.ply");
    let (_, pos) = s.raw("GET", "/api/positions", None, b"");
    assert_eq!(pos.len(), 8 * 3 * 4);
    assert_eq!(f32::from_le_bytes(pos[12..16].try_into().unwrap()), 20.0); // vertex 1, x
    let (_, idx) = s.raw("GET", "/api/indices", None, b"");
    assert_eq!(idx.len(), 12 * 3 * 4);
}

#[test]
fn a_fit_measure_and_segment_give_the_known_cube_numbers() {
    let s = start(true);
    let (st, f) = s.json("/api/fit", &json!({"kind":"plane","vertices":[4,5,6,7]}));
    assert_eq!(st, 200, "{f}");
    assert_eq!(f["fit"]["center"][2], 20.0);
    assert_eq!(f["fit"]["normal"], json!([0.0, 0.0, 1.0]));
    let (_, m) = s.json("/api/measure", &json!({"first":{"kind":"plane","vertices":[0,1,2,3]},"second":{"kind":"plane","vertices":[4,5,6,7]}}));
    assert_eq!(m["relation"]["distance"], 20.0, "{m}");
    let (_, seg) = s.json("/api/segment", &json!({"kind":"planes"}));
    assert_eq!(seg["regions"].as_array().unwrap().len(), 6);
    let ids = seg["triangle_region"].as_array().unwrap();
    assert_eq!(ids.len(), 12);
    assert!(ids.iter().all(|v| v.as_i64().unwrap() >= 0));
}

#[test]
fn bad_requests_get_plain_errors_and_the_server_keeps_working() {
    let s = start(true);
    for body in [
        json!({"kind":"plane","vertices":[0,1]}), // too few points
        json!({"kind":"plane","vertices":[0,1,2,99]}), // id outside the mesh
        json!({"kind":"plane","vertices":[0,1,"x"]}),
        json!({"kind":"plane","vertices":[0,1,2,4294967299u64]}), // would wrap to vertex 3 if cast
        json!({"kind":"torus","vertices":[0,1,2,3]}),
        json!({"kind":"plane","vertices":[0,1,2,3],"sigma":9}),
        json!({"kind":"plane","vertices":[0,1,2,3],"known_diameter":5}),
        json!({"kind":"sphere","vertices":[0,1,2,3,4],"known_diameter":-1}),
    ] {
        let (st, e) = s.json("/api/fit", &body);
        assert_eq!(st, 400, "{body}");
        assert!(e["error"].as_str().unwrap().len() > 5);
    }
    assert_eq!(s.raw("POST", "/api/fit", None, b"not json").0, 400);
    assert_eq!(s.raw("GET", "/nope", None, b"").0, 404);
    assert_eq!(s.raw("DELETE", "/", None, b"").0, 405);
    assert_eq!(s.json("/api/segment", &json!({"kind":"x"})).0, 400);
    assert_eq!(
        s.json("/api/fit", &json!({"kind":"plane","vertices":[4,5,6,7]}))
            .0,
        200
    );
}

#[test]
fn another_site_cannot_talk_to_it() {
    let s = start(true);
    for host in [
        "evil.example",
        "evil.example:80",
        &format!("127.0.0.1.evil.example:{}", s.port),
        "127.0.0.1:1",
    ] {
        let (st, _) = s.raw("GET", "/api/info", Some(host), b"");
        assert_eq!(st, 403, "{host}");
    }
    assert_eq!(
        s.raw(
            "GET",
            "/api/info",
            Some(&format!("localhost:{}", s.port)),
            b""
        )
        .0,
        200
    );
}

#[test]
fn opening_a_mesh_from_the_browser_replaces_it_and_a_bad_file_keeps_the_old_one() {
    let s = start(false);
    assert_eq!(s.raw("GET", "/api/positions", None, b"").0, 400); // nothing open yet
    let (st, _) = s.raw(
        "POST",
        "/api/open?name=a%2Fb%2Fcube.ply&unit=mm",
        None,
        CUBE.as_bytes(),
    );
    assert_eq!(st, 200);
    let info: Value = serde_json::from_slice(&s.raw("GET", "/api/info", None, b"").1).unwrap();
    assert_eq!(info["name"], "cube.ply"); // path stripped
    let (st, e) = s.raw("POST", "/api/open?name=junk.stl", None, b"not a mesh");
    assert_eq!(st, 400, "{}", String::from_utf8_lossy(&e));
    assert_eq!(s.raw("POST", "/api/open?name=x.txt", None, b"x").0, 400);
    assert_eq!(
        s.raw(
            "POST",
            "/api/open?name=c.ply&unit=furlongs",
            None,
            CUBE.as_bytes()
        )
        .0,
        400
    );
    let info: Value = serde_json::from_slice(&s.raw("GET", "/api/info", None, b"").1).unwrap();
    assert_eq!(info["vertices"], 8);
    // Units are applied: the same cube in inches is 25.4 times larger.
    s.raw(
        "POST",
        "/api/open?name=c.ply&unit=in",
        None,
        CUBE.as_bytes(),
    );
    let info: Value = serde_json::from_slice(&s.raw("GET", "/api/info", None, b"").1).unwrap();
    assert!((info["bounds"]["max"][0].as_f64().unwrap() - 508.0).abs() < 1e-3);
}

#[test]
fn a_page_from_another_site_cannot_send_commands_even_with_the_right_host() {
    let s = start(false);
    // A cross-site form or fetch to 127.0.0.1 carries the right Host but its own Origin.
    for origin in [
        "http://evil.example",
        "https://evil.example",
        "null",
        "http://127.0.0.1:1",
    ] {
        let (st, _) = s.raw_origin(
            "POST",
            "/api/open?name=cube.ply",
            None,
            Some(origin),
            CUBE.as_bytes(),
        );
        assert_eq!(st, 403, "{origin}");
    }
    let info: Value = serde_json::from_slice(&s.raw("GET", "/api/info", None, b"").1).unwrap();
    assert!(
        info["name"].is_null(),
        "a refused request must not open anything"
    );
    let own = format!("http://127.0.0.1:{}", s.port);
    assert_eq!(
        s.raw_origin(
            "POST",
            "/api/open?name=cube.ply",
            None,
            Some(&own),
            CUBE.as_bytes()
        )
        .0,
        200
    );
}

#[test]
fn the_stray_point_setting_changes_how_many_points_a_fit_keeps() {
    let s = start(true);
    // Four top corners plus one bottom corner: a deliberate stray.
    let req = |sigma: u64| json!({"kind":"plane","vertices":[4,5,6,7,0],"sigma":sigma});
    let keep_all = s.json("/api/fit", &req(0)).1;
    let tight = s.json("/api/fit", &req(1)).1;
    assert_eq!(keep_all["fit"]["used"], 5, "{keep_all}");
    assert!(tight["fit"]["used"].as_u64().unwrap() < 5, "{tight}");
}

#[test]
fn known_hole_fits_agree_between_gui_and_library() {
    use rulercraft_mesh::commands::{FitKind, FitRequest, Fitted, Selection, fit, select};
    use rulercraft_mesh::fit::Clip;
    use rulercraft_mesh::graph::MeshGraph;
    use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/known/hole-block.stl");
    let bytes = std::fs::read(path).unwrap();
    let mesh = read_mesh("hole-block.stl", &bytes, Unit::Mm, ImportLimits::default()).unwrap();
    let graph = MeshGraph::build(&mesh);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut state = State::default();
    state
        .open_bytes("hole-block.stl", &bytes, Unit::Mm)
        .unwrap();
    std::thread::spawn(move || serve(listener, Arc::new(Mutex::new(state))));
    let server = Server { port };
    for (min, max, expected_diameter) in [
        ([8.4, 13.4, 1.0], [11.6, 16.6, 9.0], 3.0),
        ([27.4, 12.4, 1.0], [32.6, 17.6, 9.0], 5.0),
        ([45.9, 10.9, 1.0], [54.1, 19.1, 9.0], 8.0),
    ] {
        let selection = Selection {
            in_box: Some((min, max)),
            ..Default::default()
        };
        let ids = select(&mesh, &graph, &selection).unwrap();
        let request = FitRequest {
            kind: FitKind::Cylinder,
            selection,
            clip: Clip::None,
            known_diameter: None,
        };
        let result = fit(&mesh, &graph, &request).unwrap();
        let Fitted::Cylinder(cylinder) = result.fitted else {
            panic!("expected cylinder")
        };
        let (status, gui) = server.json(
            "/api/fit",
            &json!({"kind":"cylinder", "vertices":ids, "sigma":0}),
        );
        assert_eq!(status, 200, "{gui}");
        assert_eq!(gui["selected"], result.selected);
        let output = &gui["fit"];
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
