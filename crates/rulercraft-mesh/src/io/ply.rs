// SPDX-License-Identifier: MIT OR Apache-2.0
//! PLY 1.0 scalar/list records, interpreted in their declared order.
use super::{ImportLimits, ReadError, check_bytes, finish, vertex};
use crate::{Mesh, MeshBuilder, Unit};

#[derive(Clone, Copy)]
enum Type {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}
impl Type {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "char" | "int8" => Self::I8,
            "uchar" | "uint8" => Self::U8,
            "short" | "int16" => Self::I16,
            "ushort" | "uint16" => Self::U16,
            "int" | "int32" => Self::I32,
            "uint" | "uint32" => Self::U32,
            "float" | "float32" => Self::F32,
            "double" | "float64" => Self::F64,
            _ => return None,
        })
    }
    fn integer(self) -> bool {
        !matches!(self, Self::F32 | Self::F64)
    }
    fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}
struct Property {
    name: String,
    count: Option<Type>,
    value: Type,
}
struct Element {
    name: String,
    count: usize,
    props: Vec<Property>,
}
#[derive(Clone, Copy)]
enum Encoding {
    Ascii,
    Le,
    Be,
}
fn header(bytes: &[u8]) -> Result<(Encoding, Vec<Element>, usize, usize), ReadError> {
    let mut offset = 0;
    let mut line = 0;
    let mut encoding = None;
    let mut elements: Vec<Element> = Vec::new();
    loop {
        line += 1;
        let error = |s: &str| ReadError(format!("line {line}: {s}"));
        let end = bytes[offset..]
            .iter()
            .position(|b| *b == b'\n')
            .ok_or_else(|| error("missing end_header or newline"))?
            + offset;
        let text =
            std::str::from_utf8(&bytes[offset..end]).map_err(|_| error("header must be ASCII"))?;
        offset = end + 1;
        let w: Vec<_> = text.split_whitespace().collect();
        if line == 1 {
            if w != ["ply"] {
                return Err(error("expected ply header"));
            }
            continue;
        }
        match w.as_slice() {
            ["comment" | "obj_info", ..] => {}
            ["format", enc, "1.0"] if encoding.is_none() && elements.is_empty() => {
                encoding = Some(match *enc {
                    "ascii" => Encoding::Ascii,
                    "binary_little_endian" => Encoding::Le,
                    "binary_big_endian" => Encoding::Be,
                    _ => return Err(error("unsupported PLY encoding")),
                });
            }
            ["element", name, count] if encoding.is_some() => {
                if elements.iter().any(|e| e.name == *name) {
                    return Err(error("duplicate element"));
                }
                let count = count.parse().map_err(|_| error("invalid element count"))?;
                elements.push(Element {
                    name: name.to_string(),
                    count,
                    props: Vec::new(),
                });
            }
            ["property", rest @ ..] => {
                let e = elements
                    .last_mut()
                    .ok_or_else(|| error("property without element"))?;
                let ty = |s| Type::parse(s).ok_or_else(|| error("unsupported scalar type"));
                let p = match rest {
                    [t, name] => Property {
                        name: name.to_string(),
                        count: None,
                        value: ty(t)?,
                    },
                    ["list", c, t, name] => {
                        let count = ty(c)?;
                        if !count.integer() {
                            return Err(error("list count must be an integer"));
                        }
                        Property {
                            name: name.to_string(),
                            count: Some(count),
                            value: ty(t)?,
                        }
                    }
                    _ => return Err(error("malformed property")),
                };
                if e.props.iter().any(|v| v.name == p.name) {
                    return Err(error("duplicate property"));
                }
                e.props.push(p);
            }
            ["end_header"] => {
                let enc = encoding.ok_or_else(|| error("missing format"))?;
                let vi = elements
                    .iter()
                    .position(|e| e.name == "vertex")
                    .ok_or_else(|| error("missing vertex element"))?;
                let fi = elements
                    .iter()
                    .position(|e| e.name == "face")
                    .ok_or_else(|| error("missing face element"))?;
                if vi >= fi {
                    return Err(error("vertex element must precede face element"));
                }
                for name in ["x", "y", "z"] {
                    if !elements[vi]
                        .props
                        .iter()
                        .any(|p| p.name == name && p.count.is_none())
                    {
                        return Err(error("vertex needs scalar x/y/z"));
                    }
                }
                let indices: Vec<_> = elements[fi]
                    .props
                    .iter()
                    .filter(|p| matches!(p.name.as_str(), "vertex_indices" | "vertex_index"))
                    .collect();
                if indices.len() != 1 || indices[0].count.is_none() || !indices[0].value.integer() {
                    return Err(error("face needs one integer vertex_indices list"));
                }
                if elements.iter().any(|e| e.count > 0 && e.props.is_empty()) {
                    return Err(error("element has no properties"));
                }
                return Ok((enc, elements, offset, line));
            }
            _ => return Err(error("malformed PLY header")),
        }
    }
}
struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    line: usize,
    enc: Encoding,
}
impl Cursor<'_> {
    fn error(&self, s: &str) -> ReadError {
        match self.enc {
            Encoding::Ascii => ReadError(format!("line {}: {s}", self.line)),
            _ => ReadError(format!("byte offset {}: {s}", self.offset)),
        }
    }
    fn space(&mut self) {
        while self
            .bytes
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.line += usize::from(self.bytes[self.offset] == b'\n');
            self.offset += 1;
        }
    }
    fn scalar(&mut self, ty: Type) -> Result<f64, ReadError> {
        if matches!(self.enc, Encoding::Ascii) {
            self.space();
            let start = self.offset;
            while self
                .bytes
                .get(self.offset)
                .is_some_and(|b| !b.is_ascii_whitespace())
            {
                self.offset += 1;
            }
            let s = std::str::from_utf8(&self.bytes[start..self.offset])
                .map_err(|_| self.error("invalid text value"))?;
            macro_rules! parse {
                ($t:ty) => {
                    s.parse::<$t>()
                        .map(|v| v as f64)
                        .map_err(|_| self.error("invalid or truncated scalar"))
                };
            }
            return match ty {
                Type::I8 => parse!(i8),
                Type::U8 => parse!(u8),
                Type::I16 => parse!(i16),
                Type::U16 => parse!(u16),
                Type::I32 => parse!(i32),
                Type::U32 => parse!(u32),
                Type::F32 => parse!(f32),
                Type::F64 => parse!(f64),
            };
        }
        let end = self
            .offset
            .checked_add(ty.size())
            .ok_or_else(|| self.error("scalar offset overflow"))?;
        let b = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| self.error("truncated scalar"))?;
        macro_rules! decode {
            ($t:ty, $n:expr) => {{
                let mut a = [0u8; $n];
                a.copy_from_slice(b);
                if matches!(self.enc, Encoding::Le) {
                    <$t>::from_le_bytes(a) as f64
                } else {
                    <$t>::from_be_bytes(a) as f64
                }
            }};
        }
        let v = match ty {
            Type::I8 => (b[0] as i8) as f64,
            Type::U8 => b[0] as f64,
            Type::I16 => decode!(i16, 2),
            Type::U16 => decode!(u16, 2),
            Type::I32 => decode!(i32, 4),
            Type::U32 => decode!(u32, 4),
            Type::F32 => decode!(f32, 4),
            Type::F64 => decode!(f64, 8),
        };
        self.offset = end;
        Ok(v)
    }
    fn index(&mut self, ty: Type) -> Result<usize, ReadError> {
        let v = self.scalar(ty)?;
        if v < 0. || v > usize::MAX as f64 {
            return Err(self.error("negative or excessive list count/index"));
        }
        Ok(v as usize)
    }
}
/// Reads a PLY mesh with explicit work limits and caller-selected length units.
pub fn read_ply(bytes: &[u8], unit: Unit, limits: ImportLimits) -> Result<Mesh, ReadError> {
    check_bytes(bytes, limits)?;
    let (enc, elements, offset, line) = header(bytes)?;
    let mut c = Cursor {
        bytes,
        offset,
        line: line + 1,
        enc,
    };
    let mut builder = MeshBuilder::new(unit);
    let mut ids = Vec::new();
    let mut triangles = 0usize;
    for e in elements {
        if e.name == "vertex" && (e.count > limits.max_vertices || e.count > u32::MAX as usize) {
            return Err(c.error("source vertex limit exceeded"));
        }
        // Every record consumes at least one byte: reject impossible counts before looping.
        if e.count > bytes.len() - c.offset {
            return Err(c.error("truncated element records"));
        }
        for _ in 0..e.count {
            let mut p = [0f32; 3];
            let mut face = Vec::new();
            for prop in &e.props {
                if let Some(count_ty) = prop.count {
                    let count = c.index(count_ty)?;
                    let width = if matches!(enc, Encoding::Ascii) {
                        1
                    } else {
                        prop.value.size()
                    };
                    if count > (bytes.len() - c.offset) / width {
                        return Err(c.error("truncated list"));
                    }
                    let is_face = e.name == "face"
                        && matches!(prop.name.as_str(), "vertex_indices" | "vertex_index");
                    if is_face {
                        if count < 3 {
                            return Err(c.error("face needs at least three indices"));
                        }
                        triangles = triangles
                            .checked_add(count - 2)
                            .ok_or_else(|| c.error("triangle count overflow"))?;
                        if triangles > limits.max_triangles {
                            return Err(c.error("triangle limit exceeded"));
                        }
                    }
                    for _ in 0..count {
                        if is_face {
                            let i = c.index(prop.value)?;
                            face.push(
                                *ids.get(i)
                                    .ok_or_else(|| c.error("face vertex index out of range"))?,
                            );
                        } else {
                            c.scalar(prop.value)?;
                        }
                    }
                } else {
                    let v = c.scalar(prop.value)?;
                    if e.name == "vertex" {
                        if let Some(axis) = ["x", "y", "z"].iter().position(|s| *s == prop.name) {
                            p[axis] = v as f32;
                        }
                    }
                }
            }
            if e.name == "vertex" {
                ids.push(
                    vertex(&mut builder, p, unit)
                        .ok_or_else(|| c.error("position or converted position is not finite"))?,
                );
            }
            if e.name == "face" {
                for k in 1..face.len() - 1 {
                    builder.triangle([face[0], face[k], face[k + 1]]);
                }
            }
        }
    }
    if matches!(enc, Encoding::Ascii) {
        c.space();
    }
    if c.offset != bytes.len() {
        return Err(c.error("undeclared trailing data"));
    }
    finish(builder)
}
