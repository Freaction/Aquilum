mod line;
mod list;

use std::ops::Range;

use aquilum_core::markdown::fence::{Fence, INDENT_LIMIT};
use aquilum_core::markdown::indent::Indent;
use aquilum_core::markdown::list as grammar;

use super::super::embed;
use super::super::highlight::Lang;
use super::super::table;
use line::{definition, indented, quote, rule, underline};
use list::Levels;

pub(super) use line::heading;
pub use list::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callout {
    Quote,
    Book,
    Other,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Block {
    #[default]
    Text,
    Heading { level: u8, marker: usize },
    Item(Item),
    Continuation { level: usize, lead: usize, width: usize },
    Quote { marker: usize, depth: u8, callout: Option<(Range<usize>, Callout)> },
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

    pub fn is_quote(&self) -> bool {
        matches!(self, Block::Quote { .. })
    }

    pub(super) fn content(&self) -> usize {
        match self {
            Block::Heading { marker, .. } | Block::Quote { marker, callout: None, .. } | Block::Definition { marker } => *marker,
            Block::Quote { callout: Some((range, _)), .. } => range.end,
            Block::Item(item) => item.content(),
            Block::Continuation { lead, .. } => *lead,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct State {
    table: bool,
    open: Option<(Fence, Levels)>,
    levels: Levels,
    lang: Option<Lang>,
    quoting: bool,
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
        let starts = !self.table && self.open.is_none() && !self.math && row(line) && next.is_some_and(table::is_separator);
        let closes = self.table && table::is_meta(line);
        let continues = self.table && row(line);
        self.table = starts || continues;
        (starts || continues || closes).then(|| Block::Table { first: starts, last: closes || !next.is_some_and(|n| row(n) || table::is_meta(n)) })
    }

    fn code_line(&mut self, line: &str, next: Option<&str>) -> Option<Block> {
        if let Some((fence, saved)) = self.open {
            let close = fence.closes(line);
            if close {
                self.open = None;
                self.levels = saved;
            }
            return Some(Block::Code { first: false, last: close || next.is_none(), fence: close, lang: self.lang });
        }
        if self.math || self.commenting {
            return None;
        }
        let limit = if self.levels.is_empty() { INDENT_LIMIT } else { usize::MAX };
        let fence = Fence::open(line, limit)?;
        self.open = Some((fence, self.levels));
        self.lang = Lang::from_info(fence.info(line));
        Some(Block::Code { first: true, last: next.is_none(), fence: true, lang: self.lang })
    }

    fn math_line(&mut self, line: &str, next: Option<&str>) -> Option<Block> {
        let marker = line.trim() == "$$";
        if self.commenting || !self.math && !marker {
            return None;
        }
        let (first, close) = (!self.math, self.math && marker);
        self.math = !close;
        Some(Block::Code { first, last: close || next.is_none(), fence: first || close, lang: None })
    }

    fn comment_line(&mut self, line: &str) -> Option<Block> {
        let tail = if self.commenting { line } else { line.trim_start().strip_prefix("<!--")? };
        self.commenting = !tail.contains("-->");
        Some(Block::Comment)
    }

    fn item_line(&mut self, line: &str) -> Option<Block> {
        let found = grammar::item(line).filter(|found| self.levels.accepts(found.indent.columns))?;
        let level = self.levels.enter(found.indent.columns, found.content_column());
        Some(Block::Item(Item::new(&found, level, line)))
    }

    fn continuation_line(&self, line: &str) -> Option<Block> {
        if line.trim().is_empty() {
            return None;
        }
        let indent = Indent::of(line);
        let level = self.levels.owner(indent.columns)?;
        Some(Block::Continuation { level, lead: indent.bytes, width: self.levels.width(level) })
    }

    fn classify(&mut self, line: &str, next: Option<&str>) -> Block {
        if let Some(block) = self.table_line(line, next).or_else(|| self.code_line(line, next)).or_else(|| self.math_line(line, next)).or_else(|| self.comment_line(line)) {
            return block;
        }
        if self.setext && underline(line).is_some() {
            Block::Underline
        } else if embed::media::is_embed(line) {
            Block::Image
        } else if let Some(h) = heading(line) {
            h
        } else if rule(line) {
            Block::Rule
        } else if let Some(i) = self.item_line(line) {
            i
        } else if let Some(q) = quote(line) {
            q
        } else if let Some(c) = self.continuation_line(line) {
            c
        } else if let Some(d) = definition(line).filter(|_| self.paragraph || self.defining) {
            d
        } else if self.levels.is_empty() && !self.paragraph && indented(line) {
            Block::Code { first: !self.indenting, last: !next.is_some_and(indented), fence: false, lang: None }
        } else if let Some(level) = next.and_then(underline).filter(|_| self.levels.is_empty() && !line.trim().is_empty()) {
            Block::Heading { level, marker: 0 }
        } else {
            Block::Text
        }
    }

    pub fn step(&mut self, line: &str, next: Option<&str>) -> Block {
        let block = match self.classify(line, next) {
            Block::Quote { marker, depth, callout: Some((_, kind)) } if self.quoting && !matches!(kind, Callout::Book) => Block::Quote { marker, depth, callout: None },
            other => other,
        };
        let blank = line.trim().is_empty();
        let in_list = matches!(block, Block::Item(_) | Block::Continuation { .. });
        self.quoting = block.is_quote();
        self.paragraph = matches!(block, Block::Text) && !blank;
        self.indenting = matches!(block, Block::Code { fence: false, lang: None, .. }) && self.open.is_none();
        self.setext = matches!(block, Block::Heading { marker: 0, .. });
        self.defining = matches!(block, Block::Definition { .. });
        if !(in_list || matches!(block, Block::Text) && blank || self.open.is_some() || matches!(block, Block::Code { fence: true, .. })) {
            self.levels = Levels::default();
        }
        block
    }
}

#[cfg(test)]
pub fn classify<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<Block> {
    let mut lines = lines.peekable();
    let mut state = State::default();
    let mut out = Vec::new();
    while let Some(line) = lines.next() {
        out.push(state.step(line, lines.peek().copied()));
    }
    out
}
