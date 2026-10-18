mod code;
mod frame;
mod line;
mod list;

use std::ops::Range;

use aquilum_core::markdown::indent::Indent;
use aquilum_core::markdown::list as grammar;
use aquilum_core::markdown::quote as syntax;

use super::super::embed;
use super::super::highlight::Lang;
use super::super::table;
use code::Open;
use line::{callout, definition, indented, rule, underline};
use list::Levels;

pub use frame::{Frame, Nest};
pub(super) use line::heading;
pub use list::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Callout {
    Quote,
    Book,
    Typed(u8),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Block {
    #[default]
    Text,
    Heading { level: u8, marker: usize },
    Item(Item),
    Callout { range: Range<usize>, kind: Callout },
    Rule,
    Code { first: bool, last: bool, fence: bool, lang: Option<Lang> },
    Table { first: bool, last: bool },
    Image,
    Meta,
    Comment,
    Underline,
    Definition { marker: usize },
}

impl Block {
    pub fn is_code(&self) -> bool {
        matches!(self, Block::Code { .. })
    }

    pub fn item(&self) -> Option<&Item> {
        match self {
            Block::Item(item) => Some(item),
            _ => None,
        }
    }

    pub fn content(&self, frame: &Frame) -> usize {
        match self {
            Block::Item(item) => item.content(),
            Block::Heading { marker, .. } | Block::Definition { marker } => frame.lead + marker,
            Block::Callout { range, .. } => range.end,
            _ => frame.lead,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct State {
    table: bool,
    open: Option<Open>,
    levels: Levels,
    quoted: u8,
    stack: [u8; 8],
    panels: u8,
    paragraph: bool,
    indenting: bool,
    math: bool,
    commenting: bool,
    setext: bool,
    defining: bool,
}

impl State {
    fn table_line(&mut self, line: &str, next: Option<&str>) -> Option<Block> {
        let row = table::is_row;
        let starts = !self.table && !self.math && row(line) && next.is_some_and(table::is_separator);
        let closes = self.table && table::is_meta(line);
        let continues = self.table && row(line);
        self.table = starts || continues;
        (starts || continues || closes).then(|| Block::Table { first: starts, last: closes || !next.is_some_and(|n| row(n) || table::is_meta(n)) })
    }

    fn comment_line(&mut self, line: &str) -> Option<Block> {
        let tail = if self.commenting { line } else { line.trim_start().strip_prefix("<!--")? };
        self.commenting = !tail.contains("-->");
        Some(Block::Comment)
    }

    fn item_at(&mut self, line: &str, at: usize) -> Option<Item> {
        let found = grammar::item(&line[at..]).filter(|found| self.levels.accepts(found.indent.columns))?;
        let level = self.levels.enter(found.indent.columns, found.content_column());
        Some(Item::new(&found, level, line, at))
    }

    fn frame(&self, line: &str) -> Frame {
        if !line.starts_with([' ', '\t', '>']) {
            return Frame::default();
        }
        let indent = Indent::of(line);
        let list = self.levels.owner(indent.columns).filter(|_| indent.bytes < line.len()).map(|level| Nest { level, width: self.levels.width(level) });
        let inset = if list.is_some() { indent.bytes } else { 0 };
        let (marker, depth) = syntax::markers(&line[inset..], usize::MAX);
        Frame { list, quote: u8::try_from(depth).unwrap_or(u8::MAX), inset, marker: inset + marker, lead: inset + marker, ..Frame::default() }
    }

    fn classify(&mut self, line: &str, next: Option<&str>) -> (Frame, Block) {
        if let Some(found) = self.fenced(line, next) {
            return found;
        }
        let frame = self.frame(line);
        let rest = &line[frame.lead..];
        let after = next.and_then(|next| frame.follow(next).map(|follow| &next[follow.lead..]));
        if let Some(block) = self.table_line(rest, after) {
            return (frame, block);
        }
        if let Some(found) = self.fence(line, frame, next) {
            return found;
        }
        if let Some(block) = self.math_line(rest, after).or_else(|| self.comment_line(rest)) {
            return (frame, block);
        }
        if !rule(line) && let Some(item) = self.item_at(line, 0) {
            return (Frame::default(), Block::Item(item));
        }
        let block = if self.setext && underline(rest).is_some() {
            Block::Underline
        } else if embed::media::is_embed(rest) {
            Block::Image
        } else if let Some(h) = heading(rest) {
            h
        } else if rule(rest) {
            Block::Rule
        } else if frame.quote > 0 && let Some(item) = self.item_at(line, frame.lead) {
            Block::Item(item)
        } else if frame.quote > 0 && let Some(c) = callout(line, frame.lead) {
            c
        } else if let Some(d) = definition(rest).filter(|_| self.paragraph || self.defining) {
            d
        } else if self.levels.is_empty() && !self.paragraph && indented(line) {
            Block::Code { first: !self.indenting, last: !after.is_some_and(indented), fence: false, lang: None }
        } else if let Some(level) = after.and_then(underline).filter(|_| self.levels.is_empty() && !rest.trim().is_empty()) {
            Block::Heading { level, marker: 0 }
        } else {
            Block::Text
        };
        (frame, block)
    }

    pub fn step(&mut self, line: &str, next: Option<&str>) -> (Frame, Block) {
        let (frame, block) = match self.classify(line, next) {
            (frame, Block::Callout { kind, .. }) if self.quoted >= frame.quote && kind != Callout::Book => (frame, Block::Text),
            other => other,
        };
        let typed = matches!(block, Block::Callout { kind: Callout::Typed(_), .. });
        let mut panels = self.stack[..usize::from(self.panels)].iter().take_while(|&&depth| depth < frame.quote || depth == frame.quote && !typed).count();
        if typed && panels < self.stack.len() {
            self.stack[panels] = frame.quote;
            panels += 1;
        }
        self.stack[panels..].fill(0);
        self.panels = panels as u8;
        let stack = &self.stack[..panels];
        let after = next.filter(|_| panels > 0).map_or(0, |next| syntax::markers(&next[Indent::of(next).bytes.min(frame.inset)..], usize::MAX).1);
        let closes = stack.iter().filter(|&&depth| usize::from(depth) > after).count() as u8;
        let frame = Frame { panels: self.panels, panel: stack.last().copied().unwrap_or(0), outer: stack.first().copied().unwrap_or(0), closes, ..frame };
        let blank = line[frame.lead..].trim().is_empty();
        self.quoted = frame.quote;
        self.paragraph = matches!(block, Block::Text) && !blank;
        self.indenting = matches!(block, Block::Code { fence: false, lang: None, .. }) && self.open.is_none();
        self.setext = matches!(block, Block::Heading { marker: 0, .. });
        self.defining = matches!(block, Block::Definition { .. });
        if !(matches!(block, Block::Item(_)) || frame.list.is_some() || matches!(block, Block::Text) && blank || self.open.is_some() || matches!(block, Block::Code { fence: true, .. })) {
            self.levels = Levels::default();
        }
        (frame, block)
    }
}

#[cfg(test)]
pub fn classify<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<(Frame, Block)> {
    let mut lines = lines.peekable();
    let mut state = State::default();
    let mut out = Vec::new();
    while let Some(line) = lines.next() {
        out.push(state.step(line, lines.peek().copied()));
    }
    out
}
