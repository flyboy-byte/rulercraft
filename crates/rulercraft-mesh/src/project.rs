// SPDX-License-Identifier: MIT OR Apache-2.0
//! Self-contained JSON projects with refitted recipes (docs/specs/project-file.md).

use crate::commands::{FitOutcome, FitRequest, fit};
use crate::graph::MeshGraph;
use crate::{ImportLimits, Mesh, Unit, read_mesh};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// Bounds on parsing, import and saved recipes.
#[derive(Debug, Clone, Copy)]
pub struct ProjectLimits {
    /// Maximum encoded JSON bytes.
    pub max_bytes: usize,
    /// Maximum fitted elements.
    pub max_elements: usize,
    /// Bounds for the embedded source mesh.
    pub import: ImportLimits,
}
impl Default for ProjectLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 << 20,
            max_elements: 128,
            import: ImportLimits {
                max_bytes: 8 << 20,
                max_vertices: 1_000_000,
                max_triangles: 2_000_000,
            },
        }
    }
}
/// An element's identity, repeatable recipe and most recently computed result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    /// Unique nonempty name, at most 128 UTF-8 bytes.
    pub id: String,
    /// Selection and fit settings, mm and radians.
    pub recipe: FitRequest,
    /// Cached result, recomputed whenever the project opens.
    pub last_result: FitOutcome,
}
#[derive(Debug, Serialize, Deserialize)]
struct Source {
    name: String,
    bytes: Vec<u8>,
    unit: String,
}
#[derive(Serialize, Deserialize)]
struct Saved {
    version: u64,
    source: Source,
    elements: Vec<Element>,
}
/// A validated imported mesh and its saved elements. No filesystem access.
#[derive(Debug)]
pub struct Project {
    source: Source,
    mesh: Mesh,
    graph: MeshGraph,
    elements: Vec<Element>,
    limits: ProjectLimits,
}
/// Reopening reports which cached results changed during refitting.
#[derive(Debug)]
pub struct Reopened {
    /// The project with current results.
    pub project: Project,
    /// Element ids whose exact typed result changed, in saved order.
    pub changed: Vec<String>,
}
/// An actionable project refusal.
#[derive(Debug)]
pub struct ProjectError(pub String);
impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ProjectError {}
fn error(message: impl ToString) -> ProjectError {
    ProjectError(message.to_string())
}

impl Project {
    /// Imports a source and starts a project with no elements.
    pub fn new(
        name: &str,
        bytes: &[u8],
        unit: Unit,
        limits: ProjectLimits,
    ) -> Result<Self, ProjectError> {
        let mesh = read_mesh(name, bytes, unit, limits.import).map_err(error)?;
        if mesh.positions.iter().flatten().any(|x| !x.is_finite()) {
            return Err(error("mesh coordinates must be finite after conversion"));
        }
        let graph = MeshGraph::build(&mesh);
        let unit = match unit {
            Unit::Mm => "mm",
            Unit::Cm => "cm",
            Unit::M => "m",
            Unit::In => "in",
        };
        Ok(Self {
            source: Source {
                name: name.into(),
                bytes: bytes.into(),
                unit: unit.into(),
            },
            mesh,
            graph,
            elements: Vec::new(),
            limits,
        })
    }
    /// The imported mesh, in millimetres.
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
    /// Saved elements, in insertion order.
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }
    /// Fits and appends an element. A failure changes nothing.
    pub fn add(&mut self, id: &str, recipe: FitRequest) -> Result<(), ProjectError> {
        if id.trim().is_empty() || id.len() > 128 {
            return Err(error(
                "element id must contain 1..128 bytes and not be blank",
            ));
        }
        if self.elements.iter().any(|e| e.id == id) {
            return Err(error(format!("duplicate element id `{id}`")));
        }
        if self.elements.len() >= self.limits.max_elements {
            return Err(error("project element limit exceeded"));
        }
        let last_result = fit(&self.mesh, &self.graph, &recipe)
            .map_err(|e| error(format!("element `{id}`: {e}")))?;
        self.elements.push(Element {
            id: id.into(),
            recipe,
            last_result,
        });
        Ok(())
    }
    /// Encodes source bytes, recipes and current results. Does not touch the filesystem.
    pub fn encode(&self) -> Result<Vec<u8>, ProjectError> {
        #[derive(Serialize)]
        struct View<'a> {
            version: u64,
            source: &'a Source,
            elements: &'a [Element],
        }
        let bytes = serde_json::to_vec(&View {
            version: 1,
            source: &self.source,
            elements: &self.elements,
        })
        .map_err(error)?;
        if bytes.len() > self.limits.max_bytes {
            return Err(error("project JSON byte limit exceeded"));
        }
        Ok(bytes)
    }
    /// Parses a saved project and recomputes every result through the shared command layer.
    pub fn decode(bytes: &[u8], limits: ProjectLimits) -> Result<Reopened, ProjectError> {
        if bytes.len() > limits.max_bytes {
            return Err(error("project JSON byte limit exceeded"));
        }
        // Inspect the version before requiring any current-version fields.
        #[derive(Deserialize)]
        struct Header {
            version: u64,
        }
        let header: Header = serde_json::from_slice(bytes).map_err(error)?;
        if header.version != 1 {
            return Err(error(format!(
                "unsupported project version {}",
                header.version
            )));
        }
        let saved: Saved = serde_json::from_slice(bytes).map_err(error)?;
        if saved.elements.len() > limits.max_elements {
            return Err(error("project element limit exceeded"));
        }
        let unit = Unit::parse(&saved.source.unit)
            .ok_or_else(|| error("source unit must be mm, cm, m or in"))?;
        let mut project = Self::new(&saved.source.name, &saved.source.bytes, unit, limits)?;
        // Validate ids before doing potentially expensive fits.
        let mut ids = HashSet::new();
        for e in &saved.elements {
            if e.id.trim().is_empty() || e.id.len() > 128 {
                return Err(error(
                    "element id must contain 1..128 bytes and not be blank",
                ));
            }
            if !ids.insert(&e.id) {
                return Err(error(format!("duplicate element id `{}`", e.id)));
            }
        }
        let mut changed = Vec::new();
        for e in saved.elements {
            project.add(&e.id, e.recipe)?;
            if project.elements.last().map(|v| &v.last_result) != Some(&e.last_result) {
                changed.push(e.id);
            }
        }
        Ok(Reopened { project, changed })
    }
}
