use std::ops::Range;

use super::super::{Kind, Span};
use super::span;

const TAGS: [(&str, Kind); 12] = [
    ("sub", Kind::Sub),
    ("sup", Kind::Sup),
    ("mark", Kind::Highlight),
    ("kbd", Kind::Code),
    ("u", Kind::Underline),
    ("ins", Kind::Underline),
    ("del", Kind::Strike),
    ("s", Kind::Strike),
    ("b", Kind::Strong),
    ("strong", Kind::Strong),
    ("i", Kind::Emphasis),
    ("em", Kind::Emphasis),
];

pub(super) fn scripts(text: &str, symbol: char, kind: Kind) -> Vec<Span> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find(symbol).map(|i| from + i) {
        let after = open + 1;
        let before = text[..open].chars().next_back();
        let lone = before != Some(symbol) && before != Some('[') && text[after..].chars().next().is_some_and(|c| c != symbol && !c.is_whitespace());
        let close = lone
            .then(|| text[after..].find(|c: char| c == symbol || c.is_whitespace()).map(|i| after + i))
            .flatten()
            .filter(|&at| at > after && text[at..].starts_with(symbol) && text[at + 1..].chars().next() != Some(symbol));
        match close {
            Some(close) => {
                found.push(span(kind, open..close + 1, after..close));
                from = close + 1;
            }
            None => from = after,
        }
    }
    found
}

pub(super) fn html_comments(text: &str) -> Vec<Span> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("<!--").map(|i| from + i) {
        let Some(close) = text[open + 4..].find("-->").map(|i| open + 4 + i + 3) else { break };
        found.push(span(Kind::Comment, open..close, open..close));
        from = close;
    }
    found
}

pub(super) fn html_pairs(text: &str) -> Vec<Span> {
    let mut found = Vec::new();
    for (name, kind) in TAGS {
        let (open, close) = (format!("<{name}>"), format!("</{name}>"));
        let mut from = 0;
        while let Some(start) = text[from..].find(&open).map(|i| from + i) {
            let inner = start + open.len();
            match text[inner..].find(&close).map(|i| inner + i) {
                Some(end) if end > inner => {
                    found.push(span(kind, start..end + close.len(), inner..end));
                    from = end + close.len();
                }
                _ => from = inner,
            }
        }
    }
    found
}

pub(super) fn footnotes(text: &str) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("[^").map(|i| from + i) {
        let label = &text[open + 2..];
        match label.find(']') {
            Some(end) if end > 0 && !label[..end].contains(char::is_whitespace) => {
                found.push(open..open + 2 + end + 1);
                from = open + 2 + end + 1;
            }
            _ => from = open + 2,
        }
    }
    found
}

pub(super) fn block_id(text: &str) -> Option<Range<usize>> {
    let body = text.trim_end();
    let caret = body.rfind('^')?;
    let id = &body[caret + 1..];
    let spaced = body[..caret].chars().next_back().is_none_or(char::is_whitespace);
    (spaced && !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then_some(caret..body.len())
}

pub(super) fn escapes(text: &str) -> Vec<Span> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at + 1 < bytes.len() {
        if bytes[at] == b'\\' && bytes[at + 1].is_ascii_punctuation() {
            found.push(span(Kind::Escape, at..at + 2, at + 1..at + 2));
            at += 2;
        } else {
            at += 1;
        }
    }
    found
}
