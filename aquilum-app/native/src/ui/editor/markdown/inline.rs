use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

const READER_SCHEMES: [&str; 2] = ["quantum-reader:", "aquilum-reader:"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Emphasis,
    Strong,
    Strike,
    Code,
    Link,
    Reader,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub kind: Kind,
    pub whole: Range<usize>,
    pub inner: Range<usize>,
    pub target: Option<(String, bool)>,
}

fn run(text: &str, at: usize) -> usize {
    let c = text.as_bytes()[at];
    text.as_bytes()[at..].iter().take_while(|b| **b == c).count()
}

pub fn inline(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut links: Vec<(Range<usize>, Kind, (String, bool), Option<Range<usize>>)> = Vec::new();
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_WIKILINKS;
    for (event, range) in Parser::new_ext(text, options).into_offset_iter() {
        if let Some((_, _, _, inner)) = links.last_mut()
            && !matches!(event, Event::End(TagEnd::Link))
        {
            let r = inner.get_or_insert(range.clone());
            *r = r.start.min(range.start)..r.end.max(range.end);
        }
        match event {
            Event::Start(Tag::Emphasis) => spans.push(Span { target: None, kind: Kind::Emphasis, inner: range.start + 1..range.end - 1, whole: range }),
            Event::Start(Tag::Strong) => spans.push(Span { target: None, kind: Kind::Strong, inner: range.start + 2..range.end - 2, whole: range }),
            Event::Start(Tag::Strikethrough) => {
                let n = run(text, range.start).min(2);
                spans.push(Span { target: None, kind: Kind::Strike, inner: range.start + n..range.end - n, whole: range });
            }
            Event::Code(_) => {
                let n = run(text, range.start);
                if range.len() > n * 2 {
                    spans.push(Span { target: None, kind: Kind::Code, inner: range.start + n..range.end - n, whole: range });
                }
            }
            Event::Start(Tag::Link { link_type, dest_url, .. }) => {
                let kind = if READER_SCHEMES.iter().any(|s| dest_url.starts_with(s)) { Kind::Reader } else { Kind::Link };
                let shown = matches!(link_type, LinkType::Inline | LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut | LinkType::WikiLink { .. });
                let wiki = matches!(link_type, LinkType::WikiLink { .. });
                links.push((if shown { range } else { 0..0 }, kind, (dest_url.to_string(), wiki), None));
            }
            Event::End(TagEnd::Link) => {
                if let Some((whole, kind, target, Some(inner))) = links.pop()
                    && !whole.is_empty()
                {
                    spans.push(Span { kind, whole, inner, target: Some(target) });
                }
            }
            _ => {}
        }
    }
    spans
}

fn is_tag_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || ('а'..='я').contains(&c.to_ascii_lowercase()) || ('А'..='Я').contains(&c) || c == 'ё' || c == 'Ё'
}

pub fn hashtags(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut prev = None;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '#' && prev.is_none_or(char::is_whitespace) {
            let end = text[i + 1..].char_indices().find(|(_, c)| !is_tag_char(*c)).map_or(text.len(), |(j, _)| i + 1 + j);
            let next = text[end..].chars().next();
            if end > i + 1 && next.is_none_or(char::is_whitespace) {
                out.push(i..end);
                while chars.peek().is_some_and(|(j, _)| *j < end) {
                    chars.next();
                }
                prev = text[..end].chars().next_back();
                continue;
            }
        }
        prev = Some(c);
    }
    out
}

