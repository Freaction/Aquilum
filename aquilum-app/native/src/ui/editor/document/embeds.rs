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
use super::super::markdown::{self, Block, Callout};
use super::super::table::{self, TableView};
use super::{Document, Embed, text};

const LABEL_SIZE: u8 = 12;

enum Fetched {
    Table,
    Grid(Fetch<Grid>),
    Media(media::EmbedLine, Fetch<Media>),
    Books(Vec<embed::BookRow>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reveal {
    Never,
    Source,
    Above,
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
        let ps = &self.paragraphs;
        let until = |pred: &dyn Fn(&Block) -> bool| (i..ps.len()).find(|j| pred(&ps[*j].block));
        let resolved = self.resolver.is_some();
        match &ps[i].block {
            Block::Table { first: true, .. } => until(&|b| matches!(b, Block::Table { last: true, .. })).map(|e| (e, Reveal::Never)),
            Block::Code { first: true, lang: Some(Lang::Dataview), .. } if resolved => until(&|b| matches!(b, Block::Code { last: true, .. })).map(|e| (e, Reveal::Source)),
            Block::Image if resolved => Some((i, Reveal::Above)),
            Block::Quote { callout: Some((_, Callout::Book)), .. } if resolved => {
                let book = |j: usize| matches!(ps[j].block, Block::Quote { callout: Some((_, Callout::Book)), .. });
                let blank = |j: usize| self.text[ps[j].range()].trim_matches(|c: char| c.is_whitespace() || c == '>').is_empty();
                let mut end = i;
                let mut next = i + 1;
                while next < ps.len() {
                    if matches!(ps[next].block, Block::Quote { callout: None, .. }) && !blank(next) || book(next) {
                        end = next;
                        next += 1;
                        continue;
                    }
                    let after = (next..ps.len()).find(|j| !blank(*j));
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
        let p = &self.paragraphs[i];
        let (end, _) = self.run_end(i)?;
        match p.embed.as_deref() {
            Some(Embed { view: View::Table(view), .. }) if p.start == start => Some((start..self.paragraphs[end].end(), view)),
            _ => None,
        }
    }

    pub(super) fn embed(&mut self, first: usize, last: usize, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>) -> Option<f64> {
        let fonts = self.markdown.as_ref()?;
        let available = f64::from(width?);
        let start = self.paragraphs[first].start;
        let source = &self.text[start..self.paragraphs[last].end()];
        let editing = self.editing.filter(|(at, _)| *at == start).map(|(_, cell)| cell);
        let lines: Vec<&str> = source.split('\n').collect();
        let (fetched, stamp) = match (&self.paragraphs[first].block, self.resolver.as_ref()) {
            (Block::Table { .. }, _) => (Fetched::Table, 0),
            (Block::Code { .. }, Some(resolver)) => {
                let closed = lines.len() > 1 && matches!(self.paragraphs[last].block, Block::Code { fence: true, .. });
                let (grid, stamp) = resolver.dataview(&lines[1..lines.len() - usize::from(closed)].join("\n"));
                (Fetched::Grid(grid), stamp)
            }
            (Block::Image, Some(resolver)) => {
                let mut line = media::parse(source);
                if let Some((_, width)) = self.resized.filter(|(at, _)| *at == start) {
                    line.params.width = Some(width);
                }
                let (fetched, stamp) = resolver.media(&line.src, line.wiki);
                if matches!(fetched, Fetch::Missing(_)) {
                    self.paragraphs[first].embed = None;
                    return None;
                }
                (Fetched::Media(line, fetched), stamp)
            }
            (Block::Quote { callout: Some(_), .. }, Some(resolver)) => {
                let mut stamps = std::hash::DefaultHasher::new();
                let rows = self.paragraphs[first..=last]
                    .iter()
                    .filter_map(|p| match &p.block {
                        Block::Quote { callout: Some((range, Callout::Book)), .. } => Some(&self.text[p.range()][range.end..]),
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
        if let Some(embed) = self.paragraphs[first].embed.as_ref().filter(|e| e.key == key) {
            return Some(embed.view.height());
        }
        let unresolved = &self.unresolved.0;
        let view = match fetched {
            Fetched::Table => View::Table(table::parse_with(&lines, editing)?.layout(available, editing, |text, wrap, align, styled| match wrap {
                Some(w) if styled => text::build(fcx, lcx, styles, text, None, Some(w as f32), align, Some((fonts, &Block::Text, None, unresolved))),
                Some(w) => (text::spec_layout(fcx, lcx, styles, fonts, &Spec { wrap: Some(w), align, ..Spec::plain(text, Font::Editor, markdown::TEXT) }), markdown::Decor::default()),
                None => (text::spec_layout(fcx, lcx, styles, fonts, &Spec::plain(text, Font::Mono(LABEL_SIZE), markdown::SYNTAX)), markdown::Decor::default()),
            })),
            Fetched::Grid(grid) => View::Drawing(embed::dataview(&grid, available, &mut |spec| text::spec_layout(fcx, lcx, styles, fonts, spec))),
            Fetched::Media(line, fetched) => View::Media(MediaView::new(&fetched, line, available, self.uncropped == Some(start))),
            Fetched::Books(rows) => View::Drawing(embed::books(&rows, available, &mut |spec| text::spec_layout(fcx, lcx, styles, fonts, spec))),
        };
        let height = view.height();
        self.paragraphs[first].embed = Some(Box::new(Embed { key, view }));
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
