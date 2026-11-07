// SPDX-License-Identifier: MIT OR Apache-2.0
//! The indexed triangle mesh every reader produces (docs/specs/mesh-import.md).

use std::collections::HashMap;

/// Length unit a file's coordinates are in. Everything is stored in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Millimetres (the default).
    Mm,
    /// Centimetres.
    Cm,
    /// Metres.
    M,
    /// Inches.
    In,
}

impl Unit {
    /// Millimetres per file unit.
    pub fn to_mm(self) -> f64 {
        match self {
            Unit::Mm => 1.0,
            Unit::Cm => 10.0,
            Unit::M => 1000.0,
            Unit::In => 25.4,
        }
    }

    /// Parses `mm`, `cm`, `m` or `in`.
    pub fn parse(s: &str) -> Option<Unit> {
        match s {
            "mm" => Some(Unit::Mm),
            "cm" => Some(Unit::Cm),
            "m" => Some(Unit::M),
            "in" => Some(Unit::In),
            _ => None,
        }
    }
}

/// Axis-aligned bounding box in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Lowest corner.
    pub min: [f32; 3],
    /// Highest corner.
    pub max: [f32; 3],
}

/// An indexed triangle mesh in millimetres with exactly-equal vertices merged.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    /// Vertex positions, mm.
    pub positions: Vec<[f32; 3]>,
    /// Triangles as three vertex indices, none repeating a vertex.
    pub triangles: Vec<[u32; 3]>,
    /// Triangles dropped because two of their corners merged into one vertex.
    pub degenerate_dropped: usize,
}

impl Mesh {
    /// The bounding box, or `None` for an empty mesh.
    pub fn bounds(&self) -> Option<Bounds> {
        let first = *self.positions.first()?;
        let mut b = Bounds {
            min: first,
            max: first,
        };
        for p in &self.positions {
            for (k, v) in p.iter().enumerate() {
                b.min[k] = b.min[k].min(*v);
                b.max[k] = b.max[k].max(*v);
            }
        }
        Some(b)
    }
}

/// Builds a [`Mesh`] from triangle corners, merging bit-identical vertices (-0 counts as 0).
#[derive(Debug)]
pub struct MeshBuilder {
    scale: f64,
    mesh: Mesh,
    index: HashMap<[u32; 3], u32>,
}

impl MeshBuilder {
    /// A builder whose input coordinates are in `unit`.
    pub fn new(unit: Unit) -> Self {
        Self {
            scale: unit.to_mm(),
            mesh: Mesh::default(),
            index: HashMap::new(),
        }
    }

    /// The vertex index for a corner given in file units; `None` if a coordinate is not finite.
    pub fn vertex(&mut self, p: [f32; 3]) -> Option<u32> {
        let mut q = [0f32; 3];
        for k in 0..3 {
            if !p[k].is_finite() {
                return None;
            }
            let v = if self.scale == 1.0 {
                p[k]
            } else {
                (f64::from(p[k]) * self.scale) as f32
            };
            q[k] = v + 0.0; // -0 becomes 0
        }
        let key = q.map(f32::to_bits);
        let next = self.mesh.positions.len() as u32;
        let id = *self.index.entry(key).or_insert(next);
        if id == next {
            self.mesh.positions.push(q);
        }
        Some(id)
    }

    /// Adds a triangle of vertex indices, dropping it if two corners are the same vertex.
    pub fn triangle(&mut self, t: [u32; 3]) {
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            self.mesh.degenerate_dropped += 1;
        } else {
            self.mesh.triangles.push(t);
        }
    }

    /// Triangles kept so far.
    pub fn triangle_count(&self) -> usize {
        self.mesh.triangles.len()
    }

    /// The finished mesh.
    pub fn finish(self) -> Mesh {
        self.mesh
    }
}
