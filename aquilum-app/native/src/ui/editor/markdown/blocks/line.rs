use aquilum_core::markdown::fence::INDENT_LIMIT;
use aquilum_core::markdown::indent::Indent;
use aquilum_core::markdown::list as grammar;
use aquilum_core::markdown::quote as syntax;

use super::{Block, Callout};

pub fn heading(line: &str) -> Option<Block> {
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

pub fn rule(line: &str) -> bool {
    let indent = Indent::of(line);
    let body = line[indent.bytes..].trim_end();
    let Some(symbol) = body.bytes().next().filter(|c| matches!(c, b'-' | b'*' | b'_')) else { return false };
    indent.columns <= INDENT_LIMIT && body.bytes().filter(|b| *b != b' ').all(|b| b == symbol) && body.bytes().filter(|b| *b == symbol).count() >= 3
}

pub fn underline(line: &str) -> Option<u8> {
    let indent = Indent::of(line);
    let body = line[indent.bytes..].trim_end();
    if indent.columns > INDENT_LIMIT || body.len() < 2 {
        return None;
    }
    if body.bytes().all(|b| b == b'=') {
        Some(1)
    } else if body.bytes().all(|b| b == b'-') {
        Some(2)
    } else {
        None
    }
}

pub fn definition(line: &str) -> Option<Block> {
    line.starts_with(": ").then_some(Block::Definition { marker: 2 })
}

pub fn indented(line: &str) -> bool {
    Indent::of(line).columns > INDENT_LIMIT && !line.trim().is_empty() && grammar::item(line).is_none()
}

pub fn quote(line: &str) -> Option<Block> {
    let found = syntax::quote(line)?;
    let callout = found.callout.map(|c| {
        let kind = match c.name.as_str() {
            "quote" => Callout::Quote,
            "book" => Callout::Book,
            _ => Callout::Other,
        };
        (c.range, kind)
    });
    Some(Block::Quote { marker: found.marker, depth: u8::try_from(found.depth).unwrap_or(u8::MAX), callout })
}
