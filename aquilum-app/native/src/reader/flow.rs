use std::ops::Range;

use super::dom::{Dom, Kind, NodeId, utf16_len};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    Paragraph,
    Heading(u8),
    Pre,
    Item,
    Cell,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub link: bool,
    pub small: bool,
}

#[derive(Clone, Debug)]
pub struct Span {
    pub range: Range<usize>,
    pub style: Style,
}

#[derive(Clone, Copy, Debug)]
pub struct Origin {
    pub byte: usize,
    pub node: NodeId,
    pub utf16: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    Start,
    Center,
    End,
}

#[derive(Debug)]
pub enum Block {
    Text { text: String, spans: Vec<Span>, origins: Vec<Origin>, role: Role, depth: u8, indent: bool, align: Option<Align> },
    Image { src: String, node: NodeId },
    Rule,
}

const SKIP: &[&str] = &["head", "script", "style", "title", "noscript"];
const BLOCKS: &[&str] = &[
    "p", "div", "section", "article", "header", "footer", "aside", "nav", "main", "body", "html", "blockquote", "ul", "ol", "li", "dl", "dt", "dd", "pre",
    "table", "thead", "tbody", "tfoot", "tr", "td", "th", "figure", "figcaption", "center", "h1", "h2", "h3", "h4", "h5", "h6", "caption", "address",
];

struct Builder<'a> {
    dom: &'a Dom,
    blocks: Vec<Block>,
    text: String,
    spans: Vec<Span>,
    origins: Vec<Origin>,
    styles: Vec<Style>,
    role: Role,
    depth: u8,
    pre: u8,
    align: Option<Align>,
}

fn heading(name: &str) -> Option<u8> {
    name.strip_prefix('h').and_then(|n| n.parse().ok()).filter(|n| (1..=6).contains(n))
}

fn align_of(dom: &Dom, node: NodeId) -> Option<Align> {
    let attr = dom.attr(node, "align").map(str::to_ascii_lowercase);
    let class = dom.attr(node, "class").unwrap_or_default();
    match attr.as_deref() {
        Some("center") => Some(Align::Center),
        Some("right") => Some(Align::End),
        Some("left") => Some(Align::Start),
        _ if class.split_whitespace().any(|c| c == "text-author" || c == "date") => Some(Align::End),
        _ => None,
    }
}

impl Builder<'_> {
    fn style(&self) -> Style {
        self.styles.last().copied().unwrap_or_default()
    }

    fn flush(&mut self) {
        let text = std::mem::take(&mut self.text);
        let spans = std::mem::take(&mut self.spans);
        let origins = std::mem::take(&mut self.origins);
        let trimmed = text.trim_end_matches([' ', '\n']).len();
        if text[..trimmed].trim().is_empty() {
            return;
        }
        let mut text = text;
        text.truncate(trimmed);
        let spans = spans.into_iter().filter_map(|s| (s.range.start < trimmed).then(|| Span { range: s.range.start..s.range.end.min(trimmed), style: s.style })).collect();
        let indent = self.role == Role::Paragraph && matches!(self.blocks.last(), Some(Block::Text { role: Role::Paragraph, .. })) && self.align.is_none();
        self.blocks.push(Block::Text { text, spans, origins, role: self.role, depth: self.depth, indent, align: self.align });
    }

    fn push_text(&mut self, node: NodeId, value: &str) {
        let style = self.style();
        let start = self.text.len();
        if self.pre > 0 {
            self.origins.push(Origin { byte: start, node, utf16: 0 });
            self.text.push_str(value);
        } else {
            let mut utf16 = 0u32;
            let mut need_origin = true;
            for ch in value.chars() {
                let width = ch.len_utf16() as u32;
                if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{c}') {
                    let blank = self.text.is_empty() || self.text.ends_with([' ', '\n']);
                    if !blank {
                        if need_origin {
                            self.origins.push(Origin { byte: self.text.len(), node, utf16 });
                            need_origin = false;
                        }
                        self.text.push(' ');
                        utf16 += width;
                        continue;
                    }
                    utf16 += width;
                    need_origin = true;
                    continue;
                }
                if need_origin {
                    self.origins.push(Origin { byte: self.text.len(), node, utf16 });
                    need_origin = false;
                }
                self.text.push(ch);
                utf16 += width;
            }
        }
        if self.text.len() > start {
            self.spans.push(Span { range: start..self.text.len(), style });
        }
    }

    fn walk(&mut self, node: NodeId) {
        let dom = self.dom;
        match &dom.node(node).kind {
            Kind::Text(value) => self.push_text(node, value),
            Kind::Element { name, .. } => {
                let name = name.to_ascii_lowercase();
                let name = name.as_str();
                if SKIP.contains(&name) {
                    return;
                }
                match name {
                    "br" => {
                        self.text.push('\n');
                        return;
                    }
                    "img" | "image" => {
                        let src = dom.attr(node, "src").or_else(|| dom.attr(node, "href")).unwrap_or_default().to_owned();
                        self.flush();
                        self.blocks.push(Block::Image { src, node });
                        return;
                    }
                    "hr" => {
                        self.flush();
                        self.blocks.push(Block::Rule);
                        return;
                    }
                    _ => {}
                }
                if BLOCKS.contains(&name) {
                    self.flush();
                    let saved = (self.role, self.depth, self.pre, self.align);
                    self.role = match name {
                        "pre" => Role::Pre,
                        "li" | "dd" => Role::Item,
                        "td" | "th" => Role::Cell,
                        _ => heading(name).map_or(self.role, Role::Heading),
                    };
                    if name == "blockquote" {
                        self.depth += 1;
                    }
                    if name == "pre" {
                        self.pre += 1;
                    }
                    if let Some(align) = align_of(dom, node).or((name == "center").then_some(Align::Center)) {
                        self.align = Some(align);
                    }
                    for &child in &dom.node(node).children {
                        self.walk(child);
                    }
                    self.flush();
                    (self.role, self.depth, self.pre, self.align) = saved;
                    return;
                }
                let mut style = self.style();
                match name {
                    "b" | "strong" | "th" => style.bold = true,
                    "i" | "em" | "cite" | "dfn" | "var" => style.italic = true,
                    "code" | "tt" | "kbd" | "samp" => style.mono = true,
                    "a" => style.link = dom.attr(node, "href").is_some(),
                    "sup" | "sub" | "small" => style.small = true,
                    _ => {}
                }
                self.styles.push(style);
                for &child in &dom.node(node).children {
                    self.walk(child);
                }
                self.styles.pop();
            }
        }
    }
}

pub fn blocks(dom: &Dom) -> Vec<Block> {
    let mut builder = Builder { dom, blocks: Vec::new(), text: String::new(), spans: Vec::new(), origins: Vec::new(), styles: Vec::new(), role: Role::Paragraph, depth: 0, pre: 0, align: None };
    builder.walk(Dom::ROOT);
    builder.flush();
    builder.blocks
}

pub fn position(text: &str, origins: &[Origin], byte: usize) -> Option<(NodeId, u32)> {
    let origin = origins.iter().rev().find(|o| o.byte <= byte)?;
    let end = byte.min(text.len());
    Some((origin.node, origin.utf16 + utf16_len(&text[origin.byte..end])))
}

pub fn byte(text: &str, origins: &[Origin], node: NodeId, utf16: u32) -> Option<usize> {
    let index = origins.iter().rposition(|o| o.node == node && o.utf16 <= utf16)?;
    let origin = origins[index];
    let limit = origins.get(index + 1).map_or(text.len(), |o| o.byte);
    let mut units = origin.utf16;
    for (offset, ch) in text[origin.byte..limit].char_indices() {
        if units >= utf16 {
            return Some(origin.byte + offset);
        }
        units += ch.len_utf16() as u32;
    }
    Some(limit)
}
