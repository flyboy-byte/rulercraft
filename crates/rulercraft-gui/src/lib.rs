// SPDX-License-Identifier: MIT OR Apache-2.0
//! The server behind the standalone GUI (docs/specs/gui.md): a small HTTP/1.1 server on the standard
//! library only. It listens on loopback, checks the Host header (so a web page on another site cannot
//! talk to it), holds one mesh, and answers with the same engine functions the CLI and MCP use.
use rulercraft_mesh::fit::{Clip, CylinderOptions, fit_cylinder, fit_plane, fit_sphere};
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::measure::{Element, relate};
use rulercraft_mesh::segment::{segment_cylinders, segment_planes};
use rulercraft_mesh::{ImportLimits, Mesh, Unit, read_mesh};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PAGE: &str = include_str!("../assets/index.html");
const APP_JS: &str = include_str!("../assets/app.js");
const GEOM_JS: &str = include_str!("../assets/geom.js");
/// Largest request body accepted (a mesh file or a JSON command), bytes.
pub const MAX_BODY: usize = 512 * 1024 * 1024;
/// Largest vertex list a fit may be given.
pub const MAX_FIT_POINTS: usize = 1_000_000;

/// The open mesh.
#[derive(Default)]
pub struct State {
    loaded: Option<Loaded>,
}
struct Loaded {
    name: String,
    mesh: Mesh,
    graph: MeshGraph,
}

impl State {
    /// Opens a mesh file from disk (used by the command line).
    pub fn open_file(&mut self, path: &str, unit: &str) -> Result<(), String> {
        let unit = Unit::parse(unit).ok_or("units must be mm, cm, m or in")?;
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        self.open_bytes(name, &bytes, unit)
    }
    /// Opens a mesh from bytes; a failure leaves the previous mesh in place.
    pub fn open_bytes(&mut self, name: &str, bytes: &[u8], unit: Unit) -> Result<(), String> {
        let name: String = name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_control())
            .take(120)
            .collect();
        let mesh = read_mesh(&name, bytes, unit, ImportLimits::default())
            .map_err(|e| format!("{name}: {e}"))?;
        let graph = MeshGraph::build(&mesh);
        self.loaded = Some(Loaded { name, mesh, graph });
        Ok(())
    }
}

struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    body: Vec<u8>,
}
struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}
impl Response {
    fn json(status: u16, v: &Value) -> Self {
        Response {
            status,
            content_type: "application/json",
            body: v.to_string().into_bytes(),
        }
    }
    fn error(status: u16, msg: &str) -> Self {
        Self::json(status, &json!({ "error": msg }))
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let (mut out, mut i) = (vec![], 0);
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Reads one request; `None` for a malformed or oversized one. `port` is the port the Host header must
/// name; any other host is refused with 403 by the caller.
fn read_request(stream: &mut TcpStream) -> Result<(Request, String, String), (u16, &'static str)> {
    let mut buf = vec![];
    let mut chunk = [0u8; 8192];
    let head_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        if buf.len() > 64 * 1024 {
            return Err((431, "request headers too large"));
        }
        let n = stream.read(&mut chunk).map_err(|_| (400, "read failed"))?;
        if n == 0 {
            return Err((400, "connection closed early"));
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = std::str::from_utf8(&buf[..head_end]).map_err(|_| (400, "headers must be text"))?;
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or("");
    let mut parts = first.split(' ');
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    if method.is_empty() || !target.starts_with('/') {
        return Err((400, "bad request line"));
    }
    let (mut host, mut origin, mut length) = (String::new(), String::new(), 0usize);
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "host" => host = v.trim().to_string(),
                "origin" => origin = v.trim().to_string(),
                "content-length" => {
                    length = v.trim().parse().map_err(|_| (400, "bad content-length"))?;
                }
                _ => {}
            }
        }
    }
    if length > MAX_BODY {
        return Err((413, "request body too large"));
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk).map_err(|_| (400, "read failed"))?;
        if n == 0 {
            return Err((400, "body shorter than content-length"));
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(length);
    let (path, q) = target.split_once('?').unwrap_or((target, ""));
    let query = q
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect();
    Ok((
        Request {
            method: method.into(),
            path: path.into(),
            query,
            body,
        },
        host,
        origin,
    ))
}

/// A browser sends `Origin` on cross-site requests; only this page's own origin may use the API.
fn origin_ok(origin: &str, port: u16) -> bool {
    origin.is_empty()
        || origin == format!("http://127.0.0.1:{port}")
        || origin == format!("http://localhost:{port}")
}

fn host_ok(host: &str, port: u16) -> bool {
    host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}")
}

fn write_response(stream: &mut TcpStream, r: &Response) {
    let reason = match r.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        r.status,
        r.content_type,
        r.body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&r.body);
}

fn summary(l: &Loaded) -> Value {
    let b = l.mesh.bounds();
    json!({"name":l.name,"vertices":l.mesh.positions.len(),"triangles":l.mesh.triangles.len(),
        "degenerate_dropped":l.mesh.degenerate_dropped,"bounds":b.map(|b|json!({"min":b.min,"max":b.max}))})
}

fn vertex_list(v: &Value, count: usize) -> Result<Vec<u32>, String> {
    let a = v
        .as_array()
        .ok_or("`vertices` must be a list of vertex ids")?;
    if a.len() > MAX_FIT_POINTS {
        return Err("too many vertices for one fit (limit 1 million)".into());
    }
    // try_from, not `as`: a huge number must be refused, not wrapped into a valid id.
    let mut ids: Vec<u32> = a
        .iter()
        .map(|x| {
            x.as_u64()
                .and_then(|i| u32::try_from(i).ok())
                .filter(|&i| (i as usize) < count)
                .ok_or_else(|| "a vertex id is not a whole number inside the mesh".to_string())
        })
        .collect::<Result<_, _>>()?;
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

fn fit_json(l: &Loaded, req: &Value) -> Result<(Value, Element), String> {
    let kind = req["kind"]
        .as_str()
        .ok_or("`kind` must be plane, sphere or cylinder")?;
    let ids = vertex_list(&req["vertices"], l.mesh.positions.len())?;
    let clip = match req.get("sigma").and_then(Value::as_u64).unwrap_or(3) {
        0 => Clip::None,
        k @ 1..=3 => Clip::Sigma(k as u8),
        _ => return Err("sigma must be 0, 1, 2 or 3".into()),
    };
    let known = match req.get("known_diameter") {
        None | Some(Value::Null) => None,
        Some(v) => Some(
            v.as_f64()
                .filter(|d| d.is_finite() && *d > 0.0)
                .ok_or("known_diameter must be above 0")?,
        ),
    };
    let fail = |e: rulercraft_mesh::fit::FitError| e.to_string();
    let (fit, element) = match kind {
        "plane" => {
            if known.is_some() {
                return Err("a known diameter only applies to a sphere or cylinder fit".into());
            }
            let p = fit_plane(&l.mesh.positions, Some(&l.graph.vertex_normals), &ids, clip)
                .map_err(fail)?;
            (
                json!({"kind":"plane","center":p.center,"normal":p.normal,"basis_u":p.basis_u,"basis_v":p.basis_v,"span_u":p.span_u,"span_v":p.span_v,"sigma":p.sigma,"form_error":p.form_error,"used":p.used,"offered":p.offered}),
                Element::from(&p),
            )
        }
        "sphere" => {
            let s = fit_sphere(&l.mesh.positions, &ids, clip, known).map_err(fail)?;
            (
                json!({"kind":"sphere","center":s.center,"radius":s.radius,"diameter":s.diameter,"sigma":s.sigma,"form_error":s.form_error,"used":s.used,"offered":s.offered,"cap_angle_deg":s.cap_angle_deg}),
                Element::from(&s),
            )
        }
        "cylinder" => {
            let o = CylinderOptions {
                clip,
                assumed_diameter: known,
                ..Default::default()
            };
            let c = fit_cylinder(&l.mesh.positions, Some(&l.graph.vertex_normals), &ids, o)
                .map_err(fail)?;
            (
                json!({"kind":"cylinder","axis_point":c.axis_point,"axis":c.axis,"radius":c.radius,"diameter":c.diameter,"length":c.length,"coverage_deg":c.coverage_deg,"sigma":c.sigma,"form_error":c.form_error,"used":c.used,"offered":c.offered}),
                Element::from(&c),
            )
        }
        _ => return Err("`kind` must be plane, sphere or cylinder".into()),
    };
    Ok((json!({"selected":ids.len(),"fit":fit}), element))
}

fn route(state: &Arc<Mutex<State>>, req: &Request) -> Response {
    let Ok(mut guard) = state.lock() else {
        return Response::error(500, "state unavailable");
    };
    let get = req.method == "GET";
    let post = req.method == "POST";
    match (req.path.as_str(), get, post) {
        ("/", true, _) => Response {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: PAGE.as_bytes().to_vec(),
        },
        ("/app.js", true, _) => Response {
            status: 200,
            content_type: "text/javascript; charset=utf-8",
            body: APP_JS.as_bytes().to_vec(),
        },
        ("/geom.js", true, _) => Response {
            status: 200,
            content_type: "text/javascript; charset=utf-8",
            body: GEOM_JS.as_bytes().to_vec(),
        },
        ("/api/open", _, true) => {
            let q = |k: &str| {
                req.query
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.as_str())
            };
            let Some(unit) = Unit::parse(q("unit").unwrap_or("mm")) else {
                return Response::error(400, "units must be mm, cm, m or in");
            };
            match guard.open_bytes(q("name").unwrap_or(""), &req.body, unit) {
                Ok(()) => guard.loaded.as_ref().map_or_else(
                    || Response::error(500, "no mesh"),
                    |l| Response::json(200, &summary(l)),
                ),
                Err(e) => Response::error(400, &e),
            }
        }
        (p, true, _) if p.starts_with("/api/") => {
            let Some(l) = guard.loaded.as_ref() else {
                return if p == "/api/info" {
                    Response::json(200, &json!({"name":null}))
                } else {
                    Response::error(400, "open a mesh first")
                };
            };
            match p {
                "/api/info" => Response::json(200, &summary(l)),
                "/api/positions" => Response {
                    status: 200,
                    content_type: "application/octet-stream",
                    body: l
                        .mesh
                        .positions
                        .iter()
                        .flatten()
                        .flat_map(|v| v.to_le_bytes())
                        .collect(),
                },
                "/api/indices" => Response {
                    status: 200,
                    content_type: "application/octet-stream",
                    body: l
                        .mesh
                        .triangles
                        .iter()
                        .flatten()
                        .flat_map(|v| v.to_le_bytes())
                        .collect(),
                },
                _ => Response::error(404, "no such endpoint"),
            }
        }
        ("/api/fit" | "/api/measure" | "/api/segment", _, true) => {
            let Some(l) = guard.loaded.as_ref() else {
                return Response::error(400, "open a mesh first");
            };
            let Ok(body) = serde_json::from_slice::<Value>(&req.body) else {
                return Response::error(400, "the request body must be JSON");
            };
            let out: Result<Value, String> = match req.path.as_str() {
                "/api/fit" => fit_json(l, &body).map(|(v, _)| v),
                "/api/measure" => fit_json(l, &body["first"]).and_then(|(a, ea)| {
                    let (b, eb) = fit_json(l, &body["second"])?;
                    let r = relate(&ea, &eb);
                    Ok(json!({"first":a,"second":b,"relation":{"distance":r.distance,"components":r.components,"angle_deg":r.angle_deg,"convention":r.convention}}))
                }),
                _ => segment_json(l, &body),
            };
            match out {
                Ok(v) => Response::json(200, &v),
                Err(e) => Response::error(400, &e),
            }
        }
        (_, false, false) => Response::error(405, "use GET or POST"),
        _ => Response::error(404, "not found"),
    }
}

/// Region id per triangle (-1 for none) and a summary per region.
fn segment_json(l: &Loaded, req: &Value) -> Result<Value, String> {
    let mut ids = vec![-1i64; l.mesh.triangles.len()];
    let regions: Vec<Value> = match req["kind"].as_str().unwrap_or("planes") {
        "planes" => segment_planes(&l.mesh, &l.graph, Default::default())
            .regions
            .iter()
            .enumerate()
            .map(|(i, r)| {
                r.triangles.iter().for_each(|&t| ids[t as usize] = i as i64);
                json!({"id":i,"kind":"plane","triangles":r.triangles.len(),"area":r.area,"center":r.plane.center,"normal":r.plane.normal,"span_u":r.plane.span_u,"span_v":r.plane.span_v})
            })
            .collect(),
        "cylinders" => segment_cylinders(&l.mesh, &l.graph, Default::default())
            .regions
            .iter()
            .enumerate()
            .map(|(i, r)| {
                r.triangles.iter().for_each(|&t| ids[t as usize] = i as i64);
                json!({"id":i,"kind":"cylinder","triangles":r.triangles.len(),"area":r.area,"axis_point":r.cylinder.axis_point,"axis":r.cylinder.axis,"radius":r.cylinder.radius,"diameter":r.cylinder.diameter,"coverage_deg":r.cylinder.coverage_deg})
            })
            .collect(),
        _ => return Err("`kind` must be planes or cylinders".into()),
    };
    Ok(json!({"regions":regions,"triangle_region":ids}))
}

fn handle(state: &Arc<Mutex<State>>, mut stream: TcpStream, port: u16) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let response = match read_request(&mut stream) {
        Err((status, msg)) => Response::error(status, msg),
        Ok((_, host, _)) if !host_ok(&host, port) => Response::error(
            403,
            "this server only answers requests for 127.0.0.1 or localhost",
        ),
        Ok((_, _, origin)) if !origin_ok(&origin, port) => {
            Response::error(403, "requests from other web pages are refused")
        }
        Ok((req, _, _)) => route(state, &req),
    };
    write_response(&mut stream, &response);
}

/// Serves requests on `listener` until the process ends. Each connection gets its own thread.
pub fn serve(listener: TcpListener, state: Arc<Mutex<State>>) {
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    for stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        std::thread::spawn(move || handle(&state, stream, port));
    }
}
