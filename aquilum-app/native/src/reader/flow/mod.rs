mod builder;

use std::ops::Range;

use super::dom::{NodeId, utf16_len};

pub use builder::blocks;

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

pub fn position(text: &str, origins: &[Origin], byte: usize) -> Option<(NodeId, u32)> {
    let origin = origins.iter().rev().find(|o| o.byte <= byte)?;
    let end = byte.min(text.len());
    Some((origin.node, origin.utf16 + utf16_len(&text[origin.byte..end])))
}

pub fn byte(text: &str, origins: &[Origin], node: NodeId, utf16: u32) -> Option<usize> {
    let index = origins.iter().rposition(|o| o.node == node && o.utf16 <= utf16)?;
    let origin = origins[index];
    let limit = origins.get(index + 1).map_or(text.len(), |o| o.byte);
    Some(origin.byte + aquilum_core::text::utf16::utf16_byte(&text[origin.byte..limit], (utf16 - origin.utf16) as usize))
}
