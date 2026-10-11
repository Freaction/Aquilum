use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

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
                let kind = if crate::reader::quotes::is_reader(&dest_url) { Kind::Reader } else { Kind::Link };
                let shown = !matches!(link_type, LinkType::ReferenceUnknown | LinkType::CollapsedUnknown | LinkType::ShortcutUnknown);
                let wiki = matches!(link_type, LinkType::WikiLink { .. });
                let target = if link_type == LinkType::Email { format!("mailto:{dest_url}") } else { dest_url.to_string() };
                links.push((if shown { range } else { 0..0 }, kind, (target, wiki), None));
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
    for span in spans.iter_mut().filter(|s| s.kind == Kind::Link && s.target.as_ref().is_some_and(|(_, wiki)| *wiki)) {
        let rest = &text[span.whole.end..];
        if let Some(close) = rest.strip_prefix('(').and_then(|r| r.find(')'))
            && crate::reader::quotes::is_reader(&rest[1..1 + close])
        {
            span.kind = Kind::Reader;
            span.target = Some((rest[1..1 + close].to_owned(), false));
            span.whole.end += close + 2;
        }
    }
    let taken: Vec<Range<usize>> = spans.iter().map(|s| s.whole.clone()).collect();
    spans.extend(bare_urls(text).into_iter().filter(|url| !taken.iter().any(|t| t.start < url.end && url.start < t.end)).map(|url| {
        let shown = &text[url.clone()];
        let target = if shown.starts_with("www.") { format!("https://{shown}") } else { shown.to_owned() };
        Span { kind: Kind::Link, whole: url.clone(), inner: url, target: Some((target, false)) }
    }));
    spans
}

fn bare_urls(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(start) = ["https://", "http://", "www."].iter().filter_map(|p| text[from..].find(p).map(|i| from + i)).min() {
        let boundary = text[..start].chars().next_back().is_none_or(|c| c.is_whitespace() || "([<*_~".contains(c));
        let mut end = text[start..].find(|c: char| c.is_whitespace() || c == '<').map_or(text.len(), |i| start + i);
        while let Some(last) = text[..end].chars().next_back() {
            let unbalanced = last == ')' && text[start..end].matches(')').count() > text[start..end].matches('(').count();
            if ".,:;!?\"'*_~".contains(last) || unbalanced {
                end -= last.len_utf8();
            } else {
                break;
            }
        }
        let host = text[start..end].trim_start_matches("https://").trim_start_matches("http://");
        if boundary && host.contains('.') && !host.starts_with('.') && end > start {
            out.push(start..end);
        }
        from = end.max(start + 1);
    }
    out
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

