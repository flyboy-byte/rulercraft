// SPDX-License-Identifier: MIT OR Apache-2.0
//! Small UTF-8 XML reader for the geometry subset of 3MF. No DTD or external entities.
use super::ReadError;
use std::borrow::Cow;
use std::collections::BTreeMap;

pub(super) struct Node {
    pub name: String,
    pub namespace: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Node>,
    pub offset: usize,
}
impl Node {
    pub fn error(&self, why: impl std::fmt::Display) -> ReadError {
        fail(self.offset, why)
    }
    pub fn attr(&self, name: &str) -> Result<&str, ReadError> {
        self.attrs
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| self.error(format!("missing {name}")))
    }
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Self> {
        self.children
            .iter()
            .filter(move |n| n.name == name && n.namespace == self.namespace)
    }
    pub fn one(&self, name: &str) -> Result<&Self, ReadError> {
        let mut found = self
            .children
            .iter()
            .filter(|n| n.name == name && n.namespace == self.namespace);
        let n = found
            .next()
            .ok_or_else(|| self.error(format!("missing {name}")))?;
        if found.next().is_some() {
            return Err(self.error(format!("duplicate {name}")));
        }
        Ok(n)
    }
}
fn fail(at: usize, why: impl std::fmt::Display) -> ReadError {
    ReadError(format!("XML byte offset {at}: {why}"))
}
fn decode(text: &str, at: usize) -> Result<String, ReadError> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i + 1..];
        let end = rest
            .find(';')
            .ok_or_else(|| fail(at, "unterminated entity"))?;
        let entity = &rest[..end];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "apos" => '\'',
            "quot" => '"',
            _ => {
                let number = if let Some(v) = entity.strip_prefix("#x") {
                    u32::from_str_radix(v, 16).ok()
                } else {
                    entity.strip_prefix('#').and_then(|v| v.parse().ok())
                };
                number
                    .and_then(char::from_u32)
                    .filter(|c| matches!(*c, '\t' | '\n' | '\r') || *c >= ' ')
                    .ok_or_else(|| fail(at, "unsupported or invalid entity"))?
            }
        };
        out.push(c);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
struct Parser<'a> {
    text: &'a str,
    pos: usize,
    nodes: usize,
    max_nodes: usize,
}
impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }
    fn ws(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
    fn skip(&mut self) -> Result<(), ReadError> {
        loop {
            self.ws();
            let end = if self.rest().starts_with("<!--") {
                Some(("-->", 4))
            } else if self.rest().starts_with("<?") {
                Some(("?>", 2))
            } else {
                None
            };
            let Some((marker, prefix)) = end else {
                return Ok(());
            };
            let n = self.rest()[prefix..]
                .find(marker)
                .ok_or_else(|| fail(self.pos, "unterminated XML comment/instruction"))?;
            self.pos += prefix + n + marker.len();
        }
    }
    fn name(&mut self) -> Result<String, ReadError> {
        let start = self.pos;
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_alphanumeric() || b"_:.-".contains(c))
        {
            self.pos += 1;
        }
        let name = &self.text[start..self.pos];
        if name.is_empty()
            || name.as_bytes()[0].is_ascii_digit()
            || name.starts_with(['-', '.'])
            || name.matches(':').count() > 1
        {
            return Err(fail(start, "invalid XML name"));
        }
        Ok(name.into())
    }
    fn node(&mut self, depth: usize, ns: &BTreeMap<String, String>) -> Result<Node, ReadError> {
        if depth > 64 || self.nodes >= self.max_nodes {
            return Err(fail(self.pos, "XML depth/node limit exceeded"));
        }
        self.nodes += 1;
        self.skip()?;
        let offset = self.pos;
        if !self.rest().starts_with('<') || self.rest().starts_with("<!") {
            return Err(fail(offset, "expected element; DTD is not supported"));
        }
        self.pos += 1;
        let full = self.name()?;
        let mut attrs = BTreeMap::new();
        let mut namespaces = Cow::Borrowed(ns);
        loop {
            let before = self.pos;
            self.ws();
            if self.rest().starts_with('>') || self.rest().starts_with("/>") {
                break;
            }
            if self.pos == before {
                return Err(fail(self.pos, "attributes need whitespace"));
            }
            let key = self.name()?;
            self.ws();
            if !self.rest().starts_with('=') {
                return Err(fail(self.pos, "expected ="));
            }
            self.pos += 1;
            self.ws();
            let quote = *self
                .text
                .as_bytes()
                .get(self.pos)
                .ok_or_else(|| fail(self.pos, "missing quoted attribute"))?;
            if quote != b'\'' && quote != b'"' {
                return Err(fail(self.pos, "attribute must be quoted"));
            }
            self.pos += 1;
            let end = self
                .rest()
                .find(char::from(quote))
                .ok_or_else(|| fail(self.pos, "unterminated attribute"))?;
            let text = &self.rest()[..end];
            if text.contains('<') {
                return Err(fail(self.pos, "< in attribute"));
            }
            let value = decode(text, self.pos)?;
            self.pos += end + 1;
            if attrs.insert(key.clone(), value.clone()).is_some() {
                return Err(fail(offset, "duplicate attribute"));
            }
            if key == "xmlns" {
                namespaces.to_mut().insert(String::new(), value);
            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                namespaces.to_mut().insert(prefix.into(), value);
            }
        }
        let (prefix, local) = full.split_once(':').unwrap_or(("", &full));
        let namespace = namespaces
            .get(prefix)
            .cloned()
            .ok_or_else(|| fail(offset, "missing namespace declaration"))?;
        let mut node = Node {
            name: local.into(),
            namespace,
            attrs,
            children: Vec::new(),
            offset,
        };
        if self.rest().starts_with("/>") {
            self.pos += 2;
            return Ok(node);
        }
        self.pos += 1;
        loop {
            self.skip()?;
            if self.rest().starts_with("</") {
                self.pos += 2;
                let end = self.name()?;
                self.ws();
                if end != full || !self.rest().starts_with('>') {
                    return Err(fail(self.pos, "mismatched closing tag"));
                }
                self.pos += 1;
                return Ok(node);
            }
            if self.rest().is_empty() {
                return Err(fail(self.pos, "truncated element"));
            }
            if self.rest().starts_with("<![CDATA[") {
                let n = self.rest()[9..]
                    .find("]]>")
                    .ok_or_else(|| fail(self.pos, "unterminated CDATA"))?;
                self.pos += 9 + n + 3;
            } else if self.rest().starts_with('<') {
                node.children.push(self.node(depth + 1, &namespaces)?);
            } else {
                let n = self.rest().find('<').unwrap_or(self.rest().len());
                decode(&self.rest()[..n], self.pos)?;
                self.pos += n;
            }
        }
    }
}
pub(super) fn parse(bytes: &[u8], max_nodes: usize) -> Result<Node, ReadError> {
    let text =
        std::str::from_utf8(bytes).map_err(|e| fail(e.valid_up_to(), "XML must be UTF-8"))?;
    if text
        .chars()
        .any(|c| c < ' ' && !matches!(c, '\t' | '\n' | '\r'))
    {
        return Err(fail(0, "invalid XML control character"));
    }
    let mut parser = Parser {
        text,
        pos: if text.starts_with('\u{feff}') { 3 } else { 0 },
        nodes: 0,
        max_nodes,
    };
    let mut ns = BTreeMap::new();
    ns.insert(String::new(), String::new());
    let root = parser.node(0, &ns)?;
    parser.skip()?;
    if !parser.rest().is_empty() {
        return Err(fail(parser.pos, "data after root element"));
    }
    Ok(root)
}
