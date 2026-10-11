use std::ops::Range;

use super::super::highlight::Lang;
use super::super::table;

const CALLOUTS: [char; 7] = ['!', '?', '&', '~', '@', '$', '%'];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callout {
    Quote,
    Book,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    level: u32,
    tabs: u32,
    width: u8,
    pub ordered: bool,
    task: u8,
    done: bool,
    tone: Option<u8>,
}

impl Item {
    pub fn level(&self) -> usize {
        self.level as usize
    }

    pub fn tabs(&self) -> usize {
        self.tabs as usize
    }

    pub fn marker(&self) -> Range<usize> {
        self.tabs()..self.tabs() + usize::from(self.width)
    }

    pub fn task(&self) -> Option<(Range<usize>, bool)> {
        let end = self.marker().end;
        (self.task > 0).then(|| (end..end + usize::from(self.task), self.done))
    }

    pub fn callout(&self) -> Option<(usize, u8)> {
        self.tone.map(|tone| (self.marker().end, tone))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Block {
    #[default]
    Text,
    Heading { level: u8, marker: usize },
    Item(Item),
    Continuation { level: usize, lead: usize, width: usize },
    Quote { marker: usize, callout: Option<(Range<usize>, Callout)> },
    Rule,
    Code { first: bool, last: bool, fence: bool, lang: Option<Lang> },
    Table { first: bool, last: bool },
    Image,
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
            Block::Heading { marker, .. } | Block::Quote { marker, callout: None } => *marker,
            Block::Quote { callout: Some((range, _)), .. } => range.end,
            Block::Item(item) => item.callout().map(|(at, _)| at + 2).or(item.task().map(|(r, _)| r.end)).unwrap_or(item.marker().end),
            Block::Continuation { lead, .. } => *lead,
            _ => 0,
        }
    }
}


fn indentation(line: &str) -> (usize, usize) {
    let (mut level, mut bytes, mut spaces) = (0, 0, 0);
    for b in line.bytes() {
        match b {
            b'\t' => {
                level += 1;
                spaces = 0;
            }
            b' ' => {
                spaces += 1;
                if spaces == 4 {
                    level += 1;
                    spaces = 0;
                }
            }
            _ => break,
        }
        bytes += 1;
    }
    (level, bytes - spaces)
}

fn fence(line: &str) -> Option<(u8, usize)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let c = *trimmed.as_bytes().first()?;
    if c != b'`' && c != b'~' {
        return None;
    }
    let n = trimmed.bytes().take_while(|b| *b == c).count();
    (n >= 3 && !(c == b'`' && trimmed[n..].contains('`'))).then_some((c, n))
}

fn heading(line: &str) -> Option<Block> {
    let n = line.bytes().take_while(|b| *b == b'#').count();
    if n == 0 || n > 6 {
        return None;
    }
    match line.as_bytes().get(n) {
        None => Some(Block::Heading { level: n as u8, marker: n }),
        Some(b' ') => Some(Block::Heading { level: n as u8, marker: n + 1 }),
        _ => None,
    }
}

fn rule(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(c) = trimmed.bytes().next().filter(|c| matches!(c, b'-' | b'*' | b'_')) else { return false };
    trimmed.bytes().filter(|b| *b != b' ').all(|b| b == c) && trimmed.bytes().filter(|b| *b == c).count() >= 3 && line.len() - line.trim_start().len() < 4
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ListState {
    level: usize,
    tabs: usize,
    width: usize,
}

fn item(line: &str, previous: Option<ListState>) -> Option<Block> {
    let (level, tabs) = indentation(line);
    if level > previous.map_or(0, |p| p.level + 1) {
        return None;
    }
    let rest = &line[tabs..];
    let (symbol, ordered) = if rest.starts_with(['-', '*', '+']) && rest[1..].starts_with(' ') {
        (1, false)
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let tail = rest.as_bytes().get(digits..digits + 2)?;
        if digits == 0 || digits > 9 || !matches!(tail, b". " | b") ") {
            return None;
        }
        (digits + 1, true)
    };
    let width = symbol + 1;
    let bytes = line[tabs + width..].as_bytes();
    let task = (bytes.len() >= 3 && bytes[0] == b'[' && bytes[2] == b']' && matches!(bytes.get(3), None | Some(b' '))).then(|| if bytes.len() > 3 { 4 } else { 3 });
    let tone = (task.is_none() && bytes.get(1) == Some(&b' '))
        .then(|| CALLOUTS.iter().position(|c| bytes.first() == Some(&(*c as u8))))
        .flatten()
        .map(|tone| tone as u8);
    let done = task.is_some() && bytes[1] != b' ';
    Some(Block::Item(Item { level: level as u32, tabs: tabs as u32, width: width as u8, ordered, task: task.unwrap_or(0), done, tone }))
}

fn continuation(line: &str, list: ListState) -> Option<Block> {
    let tabs = line.bytes().take_while(|b| *b == b'\t').count();
    let spaces = line[tabs..].bytes().take_while(|b| *b == b' ').count();
    (tabs == list.tabs && spaces == list.width && line.len() > tabs + spaces).then_some(Block::Continuation { level: list.level, lead: tabs + spaces, width: list.width })
}

fn quote(line: &str) -> Option<Block> {
    let trimmed = line.trim_start_matches(' ');
    let lead = line.len() - trimmed.len();
    if lead > 3 || !trimmed.starts_with('>') {
        return None;
    }
    let marker = lead + 1 + usize::from(trimmed[1..].starts_with(' '));
    let rest = &line[marker..];
    let callout = rest.strip_prefix("[!").and_then(|r| r.find(']')).map(|close| {
        let name = rest[2..close + 2].to_ascii_lowercase();
        let end = marker + close + 3 + usize::from(rest[close + 3..].starts_with(' '));
        let kind = match name.as_str() {
            "quote" => Callout::Quote,
            "book" => Callout::Book,
            _ => Callout::Other,
        };
        (marker..end.min(line.len()), kind)
    });
    Some(Block::Quote { marker, callout })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct State {
    table: bool,
    open: Option<(u8, usize)>,
    list: Option<ListState>,
    lang: Option<Lang>,
    quoting: bool,
}

impl State {
    pub fn step(&mut self, line: &str, next: Option<&str>) -> Block {
        let row = |l: &str| l.trim_start().starts_with('|');
        let starts = !self.table && self.open.is_none() && row(line) && next.is_some_and(table::is_separator);
        let continues = self.table && (row(line) || table::is_meta(line));
        self.table = starts || continues;
        let block = if self.table {
            Block::Table { first: starts, last: !next.is_some_and(|n| row(n) || table::is_meta(n)) }
        } else if let Some((c, n)) = self.open {
            let close = fence(line).is_some_and(|(d, m)| d == c && m >= n && line.trim().bytes().all(|b| b == c));
            if close {
                self.open = None;
            }
            Block::Code { first: false, last: close || next.is_none(), fence: close, lang: self.lang }
        } else if let Some((c, n)) = fence(line) {
            self.open = Some((c, n));
            self.lang = Lang::from_info(line.trim_start_matches([' ', '`', '~']));
            Block::Code { first: true, last: next.is_none(), fence: true, lang: self.lang }
        } else if super::super::embed::media::is_embed(line) {
            Block::Image
        } else if let Some(h) = heading(line) {
            h
        } else if rule(line) {
            Block::Rule
        } else if let Some(i) = item(line, self.list) {
            i
        } else if let Some(c) = self.list.and_then(|l| continuation(line, l)) {
            c
        } else if let Some(q) = quote(line) {
            q
        } else {
            Block::Text
        };
        let block = match block {
            Block::Quote { marker, callout: Some((_, kind)) } if self.quoting && !matches!(kind, Callout::Book) => Block::Quote { marker, callout: None },
            other => other,
        };
        self.quoting = block.is_quote();
        self.list = match &block {
            Block::Item(item) => Some(ListState { level: item.level(), tabs: item.tabs(), width: item.marker().len() }),
            Block::Continuation { .. } => self.list,
            Block::Text if line.trim().is_empty() => self.list,
            _ => None,
        };
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
