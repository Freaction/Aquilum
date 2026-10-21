use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

mod outline;

pub use outline::{find_spot, outline, Spot};
use outline::{collect, Builder};

#[derive(Debug)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub start: usize,
    pub content: Range<usize>,
    pub start_char: usize,
    pub chars: Range<usize>,
}

fn scan(source: &str) -> impl Iterator<Item = Heading> + '_ {
    let mut events = Parser::new_ext(source, Options::all()).into_offset_iter();
    std::iter::from_fn(move || {
        let mut open: Option<(u8, usize, String)> = None;
        for (event, range) in events.by_ref() {
            match event {
                Event::Start(Tag::Heading { level, .. }) => open = Some((level as u8, range.start, String::new())),
                Event::Text(text) | Event::Code(text) => {
                    if let Some((_, _, title)) = open.as_mut() {
                        title.push_str(&text);
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some((level, start, title)) = open.take() {
                        let title = title.trim().to_owned();
                        return Some(Heading { level, title, start, content: range.end..source.len(), start_char: 0, chars: 0..0 });
                    }
                }
                _ => {}
            }
        }
        None
    })
}

pub fn headings(source: &str) -> Vec<Heading> {
    let mut builder = Builder::default();
    let chars = collect(source, 0, 0, &mut builder);
    builder.finish(source.len(), chars)
}

pub fn heading_at(text: &str, offset: usize) -> Option<String> {
    let line_end = |from: usize| text[from..].find('\n').map_or(text.len(), |at| from + at + 1);
    let parsed = &text[..line_end(line_end(offset.min(text.len())))];
    scan(parsed)
        .take_while(|heading| heading.start <= offset)
        .last()
        .map(|heading| heading.title)
}

pub fn heading_near(page: &str, at: usize, previous: impl FnOnce() -> Option<String>) -> Option<String> {
    heading_at(page, at).or_else(|| previous().and_then(|text| heading_at(&text, text.len())))
}

pub fn matching<'a>(headings: &'a [Heading], wanted: &str) -> Vec<&'a Heading> {
    let needle = wanted.trim().trim_start_matches('#').trim().to_lowercase();
    headings
        .iter()
        .filter(|heading| heading.title.to_lowercase() == needle)
        .collect()
}

#[cfg(test)]
mod tests;
