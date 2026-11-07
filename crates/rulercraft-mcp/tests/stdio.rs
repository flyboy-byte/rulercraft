// SPDX-License-Identifier: MIT OR Apache-2.0
//! Drives the real stdio process with known-answer meshes and JSON-RPC.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
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
    std::fs::remove_dir_all(dir).unwrap();
}
