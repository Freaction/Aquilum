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

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Block {
    #[default]
    Text,
    Heading { level: u8, marker: usize },
    Item { level: usize, tabs: usize, marker: Range<usize>, ordered: bool, task: Option<(Range<usize>, bool)>, callout: Option<(usize, u8)> },
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
            Block::Item { marker, task, callout, .. } => callout.map(|(at, _)| at + 2).or(task.as_ref().map(|(r, _)| r.end)).unwrap_or(marker.end),
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

#[derive(Clone, Copy)]
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
    let (symbol, ordered) = if rest.starts_with("- ") {
        (1, false)
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let tail = rest.as_bytes().get(digits..digits + 2)?;
        if digits == 0 || digits > 9 || !matches!(tail, b". " | b") ") {
            return None;
        }
        (digits + 1, true)
    };
    let marker = tabs..tabs + symbol + 1;
    let after = &line[marker.end..];
    let bytes = after.as_bytes();
    let task = (bytes.len() >= 3 && bytes[0] == b'[' && bytes[2] == b']' && matches!(bytes.get(3), None | Some(b' ')))
        .then(|| (marker.end..marker.end + if bytes.len() > 3 { 4 } else { 3 }, bytes[1] != b' '));
    let callout = (task.is_none() && bytes.get(1) == Some(&b' '))
        .then(|| CALLOUTS.iter().position(|c| bytes.first() == Some(&(*c as u8))))
        .flatten()
        .map(|tone| (marker.end, tone as u8));
    Some(Block::Item { level, tabs, marker, ordered, task, callout })
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

fn image(line: &str) -> bool {
    let line = line.trim();
    (line.starts_with("![[") && line.ends_with("]]") && !line[3..line.len() - 2].contains("]]"))
        || (line.starts_with("![") && line.ends_with(')') && line.contains("]("))
}

pub fn classify<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<Block> {
    let lines: Vec<&str> = lines.collect();
    let mut out = Vec::new();
    let mut table = false;
    let mut open: Option<(u8, usize)> = None;
    let mut list: Option<ListState> = None;
    let mut lang = None;
    let mut quoting = false;
    for (index, line) in lines.iter().copied().enumerate() {
        let starts = !table && open.is_none() && line.trim_start().starts_with('|') && lines.get(index + 1).is_some_and(|next| table::is_separator(next));
        let continues = table && (line.trim_start().starts_with('|') || (table::is_meta(line) && matches!(out.last(), Some(Block::Table { .. }))));
        if !continues
            && table
            && let Some(Block::Table { last, .. }) = out.last_mut()
        {
            *last = true;
        }
        table = starts || continues;
        let block = if starts || continues {
            Block::Table { first: starts, last: false }
        } else if let Some((c, n)) = open {
            let close = fence(line).is_some_and(|(d, m)| d == c && m >= n && line.trim().bytes().all(|b| b == c));
            if close {
                open = None;
            }
            Block::Code { first: false, last: close, fence: close, lang }
        } else if let Some((c, n)) = fence(line) {
            open = Some((c, n));
            lang = Lang::from_info(line.trim_start_matches([' ', '`', '~']));
            Block::Code { first: true, last: false, fence: true, lang }
        } else if image(line) {
            Block::Image
        } else if let Some(h) = heading(line) {
            h
        } else if rule(line) {
            Block::Rule
        } else if let Some(i) = item(line, list) {
            i
        } else if let Some(c) = list.and_then(|l| continuation(line, l)) {
            c
        } else if let Some(q) = quote(line) {
            q
        } else {
            Block::Text
        };
        let block = match block {
            Block::Quote { marker, callout: Some((_, kind)) } if quoting && !matches!(kind, Callout::Book) => Block::Quote { marker, callout: None },
            other => other,
        };
        quoting = block.is_quote();
        list = match &block {
            Block::Item { level, tabs, marker, .. } => Some(ListState { level: *level, tabs: *tabs, width: marker.len() }),
            Block::Continuation { .. } => list,
            Block::Text if line.trim().is_empty() => list,
            _ => None,
        };
        out.push(block);
    }
    match out.last_mut() {
        Some(Block::Code { last, .. }) if open.is_some() => *last = true,
        Some(Block::Table { last, .. }) => *last = true,
        _ => {}
    }
    out
}
