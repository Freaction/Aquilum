use masonry::core::BrushIndex;
use masonry::kurbo::Size;
use masonry::parley::{FontContext, LayoutContext};

use crate::reader::hyphen;
use crate::reader::page::{self, Item, Settings};
use super::{Anchor, Session};

fn line_start(item: &Item, y: f64) -> usize {
    match item {
        Item::Text { layout, breaks, .. } => {
            let at = layout.lines().find(|l| f64::from(l.metrics().max_coord) > y + 0.5).or_else(|| layout.lines().last()).map_or(0, |l| l.text_range().start);
            hyphen::to_text(breaks, at)
        }
        _ => 0,
    }
}

fn line_end(item: &Item, y: f64) -> usize {
    match item {
        Item::Text { layout, breaks, .. } => {
            let at = layout.lines().filter(|l| f64::from(l.metrics().min_coord) < y - 0.5).last().map_or(0, |l| l.text_range().end);
            hyphen::to_text(breaks, at)
        }
        _ => 0,
    }
}

impl Session {
    pub fn needs_layout(&self) -> bool {
        self.laid.is_none()
    }

    pub fn ensure(&mut self, view: Size, settings: &Settings, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>) {
        let key = (view, settings.clone());
        if self.laid.is_some() && self.key.as_ref() == Some(&key) {
            return;
        }
        if self.laid.is_some() && (self.page, self.scroll) != self.placed {
            self.anchor = self.current();
        }
        self.view = view;
        let max_inline = (f64::from(settings.max_width_ch) * page::advance(fcx, lcx, settings)).round();
        self.geometry = page::geometry(view, settings, max_inline);
        let (book, section, images) = (&self.book, self.section, &mut self.images);
        let mut load = |src: &str| images.entry(src.to_owned()).or_insert_with(|| book.resource(section, src).and_then(|b| crate::images::decode(&b))).clone();
        let fit = if settings.scrolled { view.height - 2.0 * settings.margin } else { self.geometry.height };
        let items = page::layout_blocks(&self.blocks, &mut load, fcx, lcx, settings, self.geometry.column_width, fit.max(1.0));
        let height = if settings.scrolled { f64::INFINITY } else { self.geometry.height };
        self.laid = Some(page::paginate(items, height));
        self.key = Some(key);
        self.place();
    }

    pub(super) fn scrolled(&self) -> bool {
        self.key.as_ref().is_some_and(|(_, s)| s.scrolled)
    }

    pub(super) fn pages(&self) -> usize {
        let Some(laid) = &self.laid else { return 1 };
        if self.scrolled() { 1 } else { laid.columns.len().div_ceil(self.geometry.columns).max(1) }
    }

    pub(super) fn max_scroll(&self) -> f64 {
        self.laid.as_ref().map_or(0.0, |l| (l.height - self.view.height + 2.0 * self.padding()).max(0.0))
    }

    pub(super) fn padding(&self) -> f64 {
        self.key.as_ref().map_or(0.0, |(_, s)| s.margin)
    }

    fn find(&self, block: usize, byte: usize) -> Option<(usize, f64)> {
        let laid = self.laid.as_ref()?;
        let y = match laid.items.get(block)? {
            Item::Text { layout, breaks, .. } => layout.lines().find(|l| l.text_range().end > hyphen::to_layout(breaks, byte)).or_else(|| layout.lines().last()).map_or(0.0, |l| f64::from(l.metrics().min_coord)),
            _ => 0.0,
        };
        laid.columns.iter().enumerate().find_map(|(index, column)| {
            column.pieces.iter().find(|p| p.item == block && y >= p.from - 0.5 && y < p.to).map(|p| (index, p.y + y - p.from))
        })
    }

    pub(super) fn place(&mut self) {
        if self.laid.is_none() {
            return;
        }
        let pages = self.pages();
        let max = self.max_scroll();
        match self.anchor {
            Anchor::Start => (self.page, self.scroll) = (0, 0.0),
            Anchor::End => (self.page, self.scroll) = (pages - 1, max),
            Anchor::Fraction(f) => (self.page, self.scroll) = (((f * pages as f64).floor() as usize).min(pages - 1), (f * max).min(max)),
            Anchor::At(block, byte) => {
                let (column, y) = self.find(block, byte).unwrap_or((0, 0.0));
                (self.page, self.scroll) = ((column / self.geometry.columns).min(pages - 1), y.min(max));
            }
        }
        self.placed = (self.page, self.scroll);
    }

    fn current(&self) -> Anchor {
        self.start().map_or(Anchor::Start, |(block, byte)| Anchor::At(block, byte))
    }

    pub(super) fn start(&self) -> Option<(usize, usize)> {
        let laid = self.laid.as_ref()?;
        if self.scrolled() {
            let column = laid.columns.first()?;
            let piece = column.pieces.iter().find(|p| p.y + (p.to - p.from) > self.scroll)?;
            let local = piece.from + (self.scroll - piece.y).max(0.0);
            return Some((piece.item, line_start(&laid.items[piece.item], local)));
        }
        let piece = laid.columns.get(self.page * self.geometry.columns)?.pieces.first()?;
        Some((piece.item, line_start(&laid.items[piece.item], piece.from)))
    }

    pub(super) fn end(&self) -> Option<(usize, usize)> {
        let laid = self.laid.as_ref()?;
        if self.scrolled() {
            let column = laid.columns.first()?;
            let bottom = self.scroll + self.view.height - 2.0 * self.padding();
            let piece = column.pieces.iter().rev().find(|p| p.y < bottom)?;
            let local = piece.from + (bottom - piece.y).min(piece.to - piece.from);
            return Some((piece.item, line_end(&laid.items[piece.item], local)));
        }
        let first = self.page * self.geometry.columns;
        let last = (first + self.geometry.columns).min(laid.columns.len());
        let piece = laid.columns[first..last].iter().rev().find_map(|c| c.pieces.last())?;
        Some((piece.item, line_end(&laid.items[piece.item], piece.to)))
    }
}
