// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local mesh commands over MCP stdio; stdout is reserved for the protocol.
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router,
};
use rulercraft_mesh::{
    ImportLimits, Mesh, Unit,
    commands::{self, FitKind, FitRequest, Fitted, Selection},
    fit::Clip,
    graph::MeshGraph,
    measure::{Element, relate},
    read_mesh,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io::Read,
    sync::{Arc, Mutex},
};
const GUIDE: &str = "Open a local STL, PLY, OBJ or 3MF with mesh.open (file unit defaults to mm; 3MF model units win), inspect mesh.info, then select by box, near point or facing. Fit plane, sphere or cylinder using the same filters. measure fits first and second selections and reports their relation and convention. Lengths are mm and facing angles are degrees. select pages ids; reuse selection filters for fits. Undo and redo successful mesh.open operations with history.undo and history.redo; history.info reports the current position. Replaying an open requires the source file to remain unchanged. Failed commands preserve the current mesh. Segmentation and project saving are not available in this server yet.";
#[derive(Debug, Clone, Copy, Hash, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
enum FileUnit {
    Mm,
    Cm,
    M,
    In,
}
#[derive(Debug, Clone, Hash, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct OpenArgs {
    /// Local mesh file path (.stl, .ply, .obj, .3mf).
    path: String,
    /// Coordinate unit in file, default mm.
    unit: Option<FileUnit>,
    /// Lower input byte limit, maximum 2 GiB.
    max_bytes: Option<usize>,
    /// Lower source vertex limit, maximum 50 million.
    max_vertices: Option<usize>,
    /// Lower candidate triangle limit, maximum 50 million.
    max_triangles: Option<usize>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Near {
    /// Centre in mm.
    center: [f64; 3],
    /// Inclusive radius in mm.
    radius: f64,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Facing {
    /// Nonzero direction toward which the selected vertices face.
    direction: [f64; 3],
    /// Angular tolerance, degrees; default 30.
    within_deg: Option<f64>,
}
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Filters {
    /// Inclusive [minimum, maximum] corners in mm.
    in_box: Option<[[f64; 3]; 2]>,
    /// Inclusive spherical distance filter.
    near: Option<Near>,
    /// Vertex-normal direction filter; zero normals are skipped.
    facing: Option<Facing>,
}
impl Filters {
    fn selection(&self) -> Selection {
        Selection {
            in_box: self.in_box.map(|[a, b]| (a, b)),
            near: self.near.as_ref().map(|n| (n.center, n.radius)),
            facing: self
                .facing
                .as_ref()
                .map(|f| (f.direction, f.within_deg.unwrap_or(30.).to_radians())),
        }
    }
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SelectArgs {
    /// Combined filters; omitted means all vertices.
    selection: Option<Filters>,
    /// First selected vertex offset, default 0.
    offset: Option<usize>,
    /// Page size, default 256 and maximum 1024.
    limit: Option<usize>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Plane,
    Sphere,
    Cylinder,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct FitArgs {
    /// Geometric element to fit.
    kind: Kind,
    /// Combined filters; omitted means all vertices (up to 1 million).
    selection: Option<Filters>,
    /// 0 keeps all points; 1, 2 or 3 sigma clipping; default 3.
    sigma: Option<u8>,
    /// Sphere held diameter, or cylinder assumed diameter metadata, mm.
    known_diameter: Option<f64>,
}
impl FitArgs {
    fn request(&self) -> Result<FitRequest, String> {
        let clip = match self.sigma.unwrap_or(3) {
            0 => Clip::None,
            k @ 1..=3 => Clip::Sigma(k),
            _ => return Err("sigma must be 0, 1, 2 or 3".into()),
        };
        Ok(FitRequest {
            kind: match self.kind {
                Kind::Plane => FitKind::Plane,
                Kind::Sphere => FitKind::Sphere,
                Kind::Cylinder => FitKind::Cylinder,
            },
            selection: self
                .selection
                .as_ref()
                .map(Filters::selection)
                .unwrap_or_default(),
            clip,
            known_diameter: self.known_diameter,
        })
    }
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct MeasureArgs {
    /// First fit, establishing the first element's measurement convention.
    first: FitArgs,
    /// Second fit, on the same current mesh.
    second: FitArgs,
}
struct Loaded {
    path: String,
    format: &'static str,
    mesh: Mesh,
    graph: MeshGraph,
}
struct OpenStep {
    args: OpenArgs,
    source_fingerprint: u64,
}
#[derive(Default)]
struct Session {
    current: Option<Loaded>,
    history: Vec<OpenStep>,
    position: usize,
}
impl Loaded {
    fn summary(&self) -> Value {
        let bounds = self.mesh.bounds();
        json!({"path":self.path,"format":self.format,"vertices":self.mesh.positions.len(),"triangles":self.mesh.triangles.len(),"degenerate_dropped":self.mesh.degenerate_dropped,"bounds":bounds.map(|b|json!({"min":b.min,"max":b.max}))})
    }
    fn fit(&self, args: &FitArgs) -> Result<(Value, Element), String> {
        let request = args.request()?;
        let selected = commands::select(&self.mesh, &self.graph, &request.selection)
            .map_err(|e| e.to_string())?;
        if selected.len() > 1_000_000 {
            return Err(
                "fit selection exceeds the 1 million point limit; select a smaller region".into(),
            );
        }
        let out = commands::fit(&self.mesh, &self.graph, &request).map_err(|e| e.to_string())?;
        let (value, element) = match out.fitted {
            Fitted::Plane(p) => {
                let e = Element::from(&p);
                (
                    json!({"kind":"plane","center":p.center,"normal":p.normal,"basis_u":p.basis_u,"basis_v":p.basis_v,"span_u":p.span_u,"span_v":p.span_v,"sigma":p.sigma,"form_error":p.form_error,"min_distance":p.min_distance,"max_distance":p.max_distance,"used":p.used,"offered":p.offered,"rounds":p.rounds}),
                    e,
                )
            }
            Fitted::Sphere(s) => {
                let e = Element::from(&s);
                (
                    json!({"kind":"sphere","center":s.center,"radius":s.radius,"diameter":s.diameter,"sigma":s.sigma,"form_error":s.form_error,"min_residual":s.min_residual,"max_residual":s.max_residual,"used":s.used,"offered":s.offered,"rounds":s.rounds,"cap_angle_deg":s.cap_angle_deg,"free_diameter":s.free_diameter}),
                    e,
                )
            }
            Fitted::Cylinder(c) => {
                let e = Element::from(&c);
                (
                    json!({"kind":"cylinder","axis_point":c.axis_point,"axis":c.axis,"radius":c.radius,"diameter":c.diameter,"length":c.length,"coverage_deg":c.coverage_deg,"sigma":c.sigma,"form_error":c.form_error,"min_residual":c.min_residual,"max_residual":c.max_residual,"used":c.used,"offered":c.offered,"rounds":c.rounds,"assumed_diameter":c.assumed_diameter,"extend_before":c.extend_before,"extend_after":c.extend_after}),
                    e,
                )
            }
        };
        Ok((json!({"selected":out.selected,"fit":value}), element))
    }
}
fn result(value: Result<Value, String>) -> CallToolResult {
    match value {
        Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
        Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
    }
}
#[derive(Clone, Default)]
struct MeshServer {
    state: Arc<Mutex<Session>>,
}
impl MeshServer {
    fn with_mesh(&self, f: impl FnOnce(&Loaded) -> Result<Value, String>) -> CallToolResult {
        result((|| {
            let state = self
                .state
                .lock()
                .map_err(|_| "mesh state lock unavailable")?;
            f(state
                .current
                .as_ref()
                .ok_or("open a mesh with mesh.open first")?)
        })())
    }
    fn load(&self, args: &OpenArgs) -> Result<(Loaded, u64), String> {
        let defaults = ImportLimits::default();
        let lower = |value: Option<usize>, max: usize| {
            let value = value.unwrap_or(max);
            if value > max {
                Err("import limits may lower but not exceed the defaults".to_string())
            } else {
                Ok(value)
            }
        };
        let limits = ImportLimits {
            max_bytes: lower(args.max_bytes, defaults.max_bytes)?,
            max_vertices: lower(args.max_vertices, defaults.max_vertices)?,
            max_triangles: lower(args.max_triangles, defaults.max_triangles)?,
        };
        let format = rulercraft_mesh::io::format_name(&args.path).map_err(|e| e.to_string())?;
        let file = std::fs::File::open(&args.path).map_err(|e| format!("{}: {e}", args.path))?;
        let mut bytes = Vec::new();
        file.take(limits.max_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > limits.max_bytes {
            return Err(format!("input exceeds the {} byte limit", limits.max_bytes));
        }
        let source_fingerprint = fingerprint(&bytes);
        let unit = match args.unit.unwrap_or(FileUnit::Mm) {
            FileUnit::Mm => Unit::Mm,
            FileUnit::Cm => Unit::Cm,
            FileUnit::M => Unit::M,
            FileUnit::In => Unit::In,
        };
        let mesh = read_mesh(&args.path, &bytes, unit, limits).map_err(|e| e.to_string())?;
        if mesh.positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err("converted mesh position is not finite".into());
        }
        let graph = MeshGraph::build(&mesh);
        let loaded = Loaded {
            path: args.path.clone(),
            format,
            mesh,
            graph,
        };
        Ok((loaded, source_fingerprint))
    }
    fn open(&self, args: OpenArgs) -> Result<Value, String> {
        let (loaded, source_fingerprint) = self.load(&args)?;
        let summary = loaded.summary();
        let mut session = self
            .state
            .lock()
            .map_err(|_| "mesh state lock unavailable")?;
        let position = session.position;
        session.history.truncate(position);
        session.history.push(OpenStep {
            args,
            source_fingerprint,
        });
        session.position = session.history.len();
        session.current = Some(loaded);
        Ok(summary)
    }
    fn history_state(&self) -> Result<Value, String> {
        let session = self
            .state
            .lock()
            .map_err(|_| "mesh state lock unavailable")?;
        Ok(json!({
            "can_undo": session.position > 0,
            "can_redo": session.position < session.history.len(),
            "position": session.position,
            "steps": session.history.len()
        }))
    }
    fn move_history(&self, redo: bool) -> Result<Value, String> {
        let mut session = self
            .state
            .lock()
            .map_err(|_| "mesh state lock unavailable")?;
        let target = if redo {
            if session.position == session.history.len() {
                return Err("there is no command to redo".into());
            }
            session.position + 1
        } else {
            if session.position == 0 {
                return Err("there is no command to undo".into());
            }
            session.position - 1
        };
        let restored = if target == 0 {
            None
        } else {
            let step = &session.history[target - 1];
            let args = step.args.clone();
            let expected_fingerprint = step.source_fingerprint;
            let (loaded, actual_fingerprint) = self.load(&args)?;
            if actual_fingerprint != expected_fingerprint {
                return Err(format!(
                    "cannot replay mesh.open: source file `{}` has changed",
                    args.path
                ));
            }
            Some(loaded)
        };
        let mesh = restored.as_ref().map(Loaded::summary);
        session.current = restored;
        session.position = target;
        Ok(json!({
            "open": session.current.is_some(),
            "mesh": mesh,
            "history": {
                "can_undo": session.position > 0,
                "can_redo": session.position < session.history.len(),
                "position": session.position,
                "steps": session.history.len()
            }
        }))
    }
}
fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
#[tool_router]
impl MeshServer {
    #[tool(
        name = "mesh.open",
        description = "Open a local mesh file in this process. A failed open preserves the previous mesh. Returns counts and mm bounds."
    )]
    fn mesh_open(&self, Parameters(args): Parameters<OpenArgs>) -> CallToolResult {
        result(self.open(args))
    }
    #[tool(
        name = "mesh.info",
        description = "Inspect the current mesh: file format, vertex/triangle counts, dropped degenerates and mm bounds."
    )]
    fn mesh_info(&self) -> CallToolResult {
        self.with_mesh(|m| Ok(m.summary()))
    }
    #[tool(
        name = "history.info",
        description = "Report whether a mesh.open command can be undone or redone in this server session."
    )]
    fn history_info(&self) -> CallToolResult {
        result(self.history_state())
    }
    #[tool(
        name = "history.undo",
        description = "Undo the most recent successful mesh.open command. Replaying a prior open requires its source file to remain unchanged."
    )]
    fn history_undo(&self) -> CallToolResult {
        result(self.move_history(false))
    }
    #[tool(
        name = "history.redo",
        description = "Redo the next mesh.open command. Replaying an open requires its source file to remain unchanged."
    )]
    fn history_redo(&self) -> CallToolResult {
        result(self.move_history(true))
    }
    #[tool(
        description = "Select vertices by combined box, near-point and normal-facing filters. Returns a bounded page of ids; reuse filters in fit."
    )]
    fn select(&self, Parameters(args): Parameters<SelectArgs>) -> CallToolResult {
        self.with_mesh(|m|{let request=args.selection.as_ref().map(Filters::selection).unwrap_or_default();let ids=commands::select(&m.mesh,&m.graph,&request).map_err(|e|e.to_string())?;
            let limit=args.limit.unwrap_or(256);if limit>1024 {return Err("selection page limit must be at most 1024".into());}let offset=args.offset.unwrap_or(0);let page:Vec<_>=ids.iter().skip(offset).take(limit).copied().collect();
            Ok(json!({"total":ids.len(),"offset":offset,"returned":page.len(),"truncated":offset.saturating_add(page.len())<ids.len(),"indices":page}))})
    }
    #[tool(
        description = "Fit a plane, sphere or cylinder to selected vertices, returning compact dimensions, pose and residual statistics in mm/degrees."
    )]
    fn fit(&self, Parameters(args): Parameters<FitArgs>) -> CallToolResult {
        self.with_mesh(|m| m.fit(&args).map(|(v, _)| v))
    }
    #[tool(
        description = "Fit two selections on the current mesh and measure their relation. Returns both fits and the distance/angle convention."
    )]
    fn measure(&self, Parameters(args): Parameters<MeasureArgs>) -> CallToolResult {
        self.with_mesh(|m|{let (first,a)=m.fit(&args.first)?;let (second,b)=m.fit(&args.second)?;let r=relate(&a,&b);
            if r.distance.into_iter().chain(r.angle_deg).chain(r.components.into_iter().flatten()).any(|v|!v.is_finite()) {return Err("measurement produced a non-finite result".into());}
            Ok(json!({"first":first,"second":second,"relation":{"distance":r.distance,"components":r.components,"angle_deg":r.angle_deg,"convention":r.convention}}))})
    }
    #[tool(
        description = "Read the short guide to mesh opening, inspection, selection, fitting and measurement, with units and limits."
    )]
    fn guide(&self) -> CallToolResult {
        result(Ok(json!({"guide":GUIDE})))
    }
}
#[tool_handler]
impl ServerHandler for MeshServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(GUIDE)
            .with_server_info(Implementation::new(
                "rulercraft-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let status = async {
        let server = MeshServer::default()
            .serve(rmcp::transport::stdio())
            .await?;
        server.waiting().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    if let Err(error) = status {
        eprintln!("rulercraft-mcp: {error}");
        std::process::exit(1);
    }
}
