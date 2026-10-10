use std::hash::{Hash, Hasher};
use std::ops::Range;

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
                        let name = header.trim().trim_start_matches("[[").trim_end_matches("]]");
                        let target = name.split('|').next().unwrap_or(name).to_owned();
                        let title = name.rsplit('|').next().unwrap_or(name).trim_end_matches(".md").to_owned();
                        let (book, stamp) = resolver.book(&target);
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
        let key = hasher.finish();
        if let Some(embed) = self.paragraphs[first].embed.as_ref().filter(|e| e.key == key) {
            return Some(embed.view.height());
        }
        let mut make = |spec: &Spec| text::spec_layout(fcx, lcx, styles, fonts, spec);
        let view = match fetched {
            Fetched::Table => View::Table(table::parse(&lines)?.layout(available, editing, |text, wrap, align, link| {
                let links = [0..text.len()];
                let spec = match wrap {
                    Some(w) => Spec { wrap: Some(w), align, links: if link { &links } else { &[] }, underline: link, ..Spec::plain(text, Font::Editor, markdown::TEXT) },
                    None => Spec::plain(text, Font::Mono(LABEL_SIZE), markdown::SYNTAX),
                };
                make(&spec)
            })),
            Fetched::Grid(grid) => View::Drawing(embed::dataview(&grid, available, &mut make)),
            Fetched::Media(line, fetched) => View::Media(MediaView::new(&fetched, line, available, self.uncropped == Some(start))),
            Fetched::Books(rows) => View::Drawing(embed::books(&rows, available, &mut make)),
        };
        let height = view.height();
        self.paragraphs[first].embed = Some(Box::new(Embed { key, view }));
        Some(height)
    }
}
