use std::hash::{Hash, Hasher};
use std::ops::Range;

use aquilum_core::search::paths::strip_markdown_extension;
use aquilum_core::search::wiki::target::split;
use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::super::embed::{self, Font, MediaView, Spec, View};
use super::super::embed::media;
use super::super::highlight::Lang;
use super::super::resolve::{Fetch, Grid, Media};
use super::super::markdown::{self, Block, Callout, Frame};
use super::super::table::{self, TableView};
use super::{Document, Embed, text};

const LABEL_SIZE: u8 = 12;

enum Fetched {
    Table,
    Grid(Fetch<Grid>),
    Media(media::EmbedLine, Fetch<Media>),
    Books(Vec<embed::BookRow>),
    Math(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reveal {
    Never,
    Source,
    Above,
    Below,
}

fn book_header(header: &str) -> (bool, String, String) {
    let header = header.trim();
    let Some(rest) = header.strip_prefix("[[") else {
        return (false, header.to_owned(), strip_markdown_extension(header).to_owned());
    };
    let link = split(rest.find("]]").map_or(rest, |close| &rest[..close]));
    (true, link.note.to_owned(), strip_markdown_extension(link.label.unwrap_or(link.note)).to_owned())
}

impl Document {
    pub(super) fn run_end(&self, i: usize) -> Option<(usize, Reveal)> {
        let count = self.lines.count();
        let block = |j: usize| &self.blocks.get(j).1;
        let until = |pred: &dyn Fn(&Block) -> bool| (i..count).find(|j| pred(block(*j)));
        let resolved = self.resolver.is_some();
        match block(i) {
            Block::Table { first: true, .. } => until(&|b| matches!(b, Block::Table { last: true, .. })).map(|e| (e, Reveal::Never)),
            Block::Code { first: true, lang: Some(Lang::Math), .. } => until(&|b| matches!(b, Block::Code { last: true, .. })).map(|e| (e, Reveal::Below)),
            Block::Code { first: true, lang: Some(Lang::Dataview), .. } if resolved => until(&|b| matches!(b, Block::Code { last: true, .. })).map(|e| (e, Reveal::Source)),
            Block::Image if resolved => Some((i, Reveal::Above)),
            Block::Callout { kind: Callout::Book, .. } if resolved => {
                let book = |j: usize| matches!(block(j), Block::Callout { kind: Callout::Book, .. });
                let quoted = |j: usize| self.blocks.get(j).0.quote > 0 && !matches!(block(j), Block::Callout { .. });
                let blank = |j: usize| self.text[self.lines.range(j)].trim_matches(|c: char| c.is_whitespace() || c == '>').is_empty();
                let mut end = i;
                let mut next = i + 1;
                while next < count {
                    if quoted(next) && !blank(next) || book(next) {
                        end = next;
                        next += 1;
                        continue;
                    }
                    let after = (next..count).find(|j| !blank(*j));
                    match after.filter(|j| book(*j)) {
                        Some(j) => next = j,
                        None => break,
                    }
                }
                Some((end, Reveal::Source))
            }
            _ => None,
        }
    }

    pub fn table_at(&self, start: usize) -> Option<(Range<usize>, &TableView)> {
        let i = self.at_offset(start);
        let (end, _) = self.run_end(i)?;
        match self.embeds.get(i) {
            Some(Embed { view: View::Table(view), .. }) if self.start(i) == start => Some((start..self.base + self.lines.end(end), view)),
            _ => None,
        }
    }

    pub(super) fn embed(&mut self, first: usize, last: usize, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>) -> Option<f64> {
        let fonts = self.markdown.as_ref()?;
        let (frame, head) = (self.view(first), &self.blocks.get(first).1);
        let line = self.line(first);
        let x = match head {
            Block::Callout { .. } => frame.bar(line, frame.quote, fonts),
            _ => frame.x(line, fonts),
        };
        let available = (f64::from(width?) - x - frame.right()).max(0.0);
        let start = self.start(first);
        let source = &self.text[self.lines.start(first)..self.lines.end(last)];
        let editing = self.editing.filter(|(at, _)| *at == start).map(|(_, cell)| cell);
        let lines: Vec<&str> = (first..=last).map(|k| &self.line(k)[self.blocks.get(k).0.lead..]).collect();
        let (fetched, stamp) = match (head, self.resolver.as_ref()) {
            (Block::Table { .. }, _) => (Fetched::Table, 0),
            (Block::Code { lang, .. }, resolver) => {
                let closed = lines.len() > 1 && matches!(self.blocks.get(last).1, Block::Code { fence: true, .. });
                let body = lines[1..lines.len() - usize::from(closed)].join("\n");
                match (lang, resolver) {
                    (Some(Lang::Math), _) => (Fetched::Math(body), 0),
                    (_, Some(resolver)) => {
                        let (grid, stamp) = resolver.dataview(&body);
                        (Fetched::Grid(grid), stamp)
                    }
                    _ => return None,
                }
            }
            (Block::Image, Some(resolver)) => {
                let mut line = media::parse(lines[0]);
                if let Some((_, width)) = self.resized.filter(|(at, _)| *at == start) {
                    line.params.resize(width);
                }
                let (fetched, stamp) = resolver.media(&line.src, line.wiki);
                if matches!(fetched, Fetch::Missing(_)) {
                    self.embeds.remove(first);
                    return None;
                }
                (Fetched::Media(line, fetched), stamp)
            }
            (Block::Callout { .. }, Some(resolver)) => {
                let mut stamps = std::hash::DefaultHasher::new();
                let rows = (first..=last)
                    .filter_map(|k| match &self.blocks.get(k).1 {
                        Block::Callout { range, kind: Callout::Book } => Some(&self.line(k)[range.end..]),
                        _ => None,
                    })
                    .map(|header| {
                        let (wiki, target, title) = book_header(header);
                        let (mut book, stamp) = resolver.book(&target);
                        if let Fetch::Ready(book) = &mut book {
                            book.linked |= wiki;
                        }
                        stamp.hash(&mut stamps);
                        embed::BookRow { book, target, title }
                    })
                    .collect();
                (Fetched::Books(rows), stamps.finish())
            }
            _ => return None,
        };
        let mut hasher = std::hash::DefaultHasher::new();
        source.hash(&mut hasher);
        available.to_bits().hash(&mut hasher);
        editing.hash(&mut hasher);
        (self.uncropped == Some(start)).hash(&mut hasher);
        self.resized.filter(|(at, _)| *at == start).map(|(_, w)| w.to_bits()).hash(&mut hasher);
        stamp.hash(&mut hasher);
        self.unresolved.1.hash(&mut hasher);
        let key = hasher.finish();
        let context = self.context(fonts);
        if let Some(embed) = self.embeds.get(first).filter(|e| e.key == key && !context.resolver.is_some_and(|r| e.view.decors().any(|decor| decor.stale(r)))) {
            return Some(embed.view.height());
        }
        let view = match fetched {
            Fetched::Table => View::Table(table::parse_with(&lines, editing)?.layout(available, editing, |text, wrap, align, styled| match wrap {
                Some(w) if styled => text::build(fcx, lcx, styles, text, None, Some(w as f32), align, Some((&context, &Frame::default(), &Block::Text, None))),
                Some(w) => (text::spec_layout(fcx, lcx, styles, fonts, &Spec { wrap: Some(w), align, ..Spec::plain(text, Font::Editor, markdown::TEXT) }), markdown::Decor::default()),
                None => (text::spec_layout(fcx, lcx, styles, fonts, &Spec::plain(text, Font::Mono(LABEL_SIZE), markdown::SYNTAX)), markdown::Decor::default()),
            })),
            Fetched::Grid(grid) => View::Drawing(embed::dataview(&grid, available, &mut |spec| text::spec_layout(fcx, lcx, styles, fonts, spec))),
            Fetched::Media(line, fetched) => View::Media(MediaView::new(&fetched, line, available, self.uncropped == Some(start))),
            Fetched::Books(rows) => View::Drawing(embed::books(&rows, available, &mut |spec| text::spec_layout(fcx, lcx, styles, fonts, spec))),
            Fetched::Math(body) => match embed::math(&body, f64::from(fonts.size), available) {
                Some(drawing) => View::Drawing(drawing),
                None => {
                    self.embeds.remove(first);
                    return None;
                }
            },
        };
        let height = view.height();
        self.embeds.insert(first, Embed { key, view, x });
        Some(height)
    }
}

#[cfg(test)]
mod tests {
    use super::book_header;

    #[test]
    fn book_headers_split_note_alias_and_extension() {
        assert_eq!(book_header(" [[Книга]] "), (true, "Книга".into(), "Книга".into()));
        assert_eq!(book_header("[[Папка/Книга.MD|Красивое имя]]"), (true, "Папка/Книга.MD".into(), "Красивое имя".into()));
        assert_eq!(book_header("[[Книга.md#Глава 1]]"), (true, "Книга.md".into(), "Книга".into()));
        assert_eq!(book_header("[[a|b|c]]"), (true, "a".into(), "b|c".into()));
        assert_eq!(book_header("[[Книга]] и ещё"), (true, "Книга".into(), "Книга".into()));
        assert_eq!(book_header("C# in depth.MD"), (false, "C# in depth.MD".into(), "C# in depth".into()));
    }
}
