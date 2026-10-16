mod marks;
mod tags;
mod urls;

use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

use crate::links::reader::is_reader;

pub use tags::hashtags;
pub use urls::bare_urls;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Emphasis,
    Strong,
    Strike,
    Sub,
    Sup,
    Underline,
    Escape,
    Code,
    Math,
    Highlight,
    Comment,
    Footnote,
    BlockId,
    Link,
    Reader,
    Image,
}

fn is_picture(source: &str, wiki: bool) -> bool {
    use crate::images::{MediaKind, extension, kind};
    match kind(source) {
        Some(found) => found == MediaKind::Image,
        None => !wiki && extension(source).is_none(),
    }
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

fn spanned(kind: Kind, whole: Range<usize>, marker: usize) -> Span {
    Span { target: None, kind, inner: whole.start + marker..whole.end - marker, whole }
}

pub fn inline(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut links: Vec<(Range<usize>, Kind, (String, bool), Option<Range<usize>>)> = Vec::new();
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_WIKILINKS | Options::ENABLE_MATH;
    for (event, range) in Parser::new_ext(text, options).into_offset_iter() {
        if let Some((_, _, _, inner)) = links.last_mut()
            && !matches!(event, Event::End(TagEnd::Link | TagEnd::Image))
        {
            let r = inner.get_or_insert(range.clone());
            *r = r.start.min(range.start)..r.end.max(range.end);
        }
        match event {
            Event::Start(Tag::Emphasis) => spans.push(spanned(Kind::Emphasis, range, 1)),
            Event::Start(Tag::Strong) => spans.push(spanned(Kind::Strong, range, 2)),
            Event::Start(Tag::Strikethrough) if run(text, range.start) >= 2 => spans.push(spanned(Kind::Strike, range, 2)),
            Event::Code(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {
                let n = run(text, range.start);
                if range.len() > n * 2 {
                    let kind = if matches!(event, Event::Code(_)) { Kind::Code } else { Kind::Math };
                    spans.push(spanned(kind, range, n));
                }
            }
            Event::Start(Tag::Link { link_type, dest_url, .. }) => {
                let kind = if is_reader(&dest_url) { Kind::Reader } else { Kind::Link };
                let shown = !matches!(link_type, LinkType::ReferenceUnknown | LinkType::CollapsedUnknown | LinkType::ShortcutUnknown);
                let wiki = matches!(link_type, LinkType::WikiLink { .. });
                let target = if link_type == LinkType::Email { format!("mailto:{dest_url}") } else { dest_url.to_string() };
                links.push((if shown { range } else { 0..0 }, kind, (target, wiki), None));
            }
            Event::Start(Tag::Image { link_type, dest_url, .. }) => {
                let wiki = matches!(link_type, LinkType::WikiLink { .. });
                let note = std::path::Path::new(dest_url.as_ref()).extension().is_none_or(|e| e.eq_ignore_ascii_case("md"));
                let kind = if is_picture(&dest_url, wiki) { Kind::Image } else { Kind::Link };
                let shown = kind == Kind::Image || wiki && note;
                links.push((if shown { range } else { 0..0 }, kind, (dest_url.to_string(), true), None));
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                if let Some((whole, kind, target, inner)) = links.pop()
                    && !whole.is_empty()
                {
                    match (kind, inner) {
                        (Kind::Image, inner) => spans.push(Span { kind, inner: inner.unwrap_or(whole.clone()), whole, target: None }),
                        (_, Some(inner)) => spans.push(Span { kind, whole, inner, target: Some(target) }),
                        _ => {}
                    }
                }
            }
            Event::Html(html) | Event::InlineHtml(html) if crate::ui::editor::embed::media::is_html_image(&html) => {
                let whole = range.start..range.start + html.find('>').map_or(html.len(), |end| end + 1);
                spans.push(Span { kind: Kind::Image, inner: whole.clone(), whole, target: None });
            }
            _ => {}
        }
    }
    for span in spans.iter_mut().filter(|s| s.kind == Kind::Link && s.target.as_ref().is_some_and(|(_, wiki)| *wiki)) {
        let rest = &text[span.whole.end..];
        if let Some(close) = rest.strip_prefix('(').and_then(|r| r.find(')'))
            && is_reader(&rest[1..1 + close])
        {
            span.kind = Kind::Reader;
            span.target = Some((rest[1..1 + close].to_owned(), false));
            span.whole.end += close + 2;
        }
    }
    let literal: Vec<Range<usize>> = spans.iter().filter(|s| matches!(s.kind, Kind::Code | Kind::Math | Kind::Link | Kind::Reader | Kind::Image)).map(|s| s.whole.clone()).collect();
    let taken: Vec<Range<usize>> = spans.iter().map(|s| s.whole.clone()).collect();
    spans.extend(bare_urls(text).into_iter().filter(|url| !taken.iter().any(|t| t.start < url.end && url.start < t.end)).map(|url| {
        let shown = &text[url.clone()];
        let target = if shown.starts_with("www.") { format!("https://{shown}") } else { shown.to_owned() };
        Span { kind: Kind::Link, whole: url.clone(), inner: url, target: Some((target, false)) }
    }));
    spans.extend(marks::extra(text, &literal));
    spans
}
