use std::collections::HashMap;
use std::path::Path;

use masonry::core::BrushIndex;
use masonry::kurbo::{Point as Pos, Rect, Size};
use masonry::parley::{Affinity, Cursor, Selection};
use masonry::parley::{FontContext, LayoutContext};
use masonry::peniko::ImageBrush;

use super::cfi::{self, Point};
use super::flow::{self, Block};
use super::hyphen;
use super::page::{self, Column, Geometry, Item, Laid, Settings};
use super::Book;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spot {
    pub block: usize,
    pub byte: usize,
}

pub enum Target {
    Start,
    Cfi(String),
    Fraction(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Anchor {
    Start,
    End,
    At(usize, usize),
    Fraction(f64),
}

pub struct Session {
    book: Book,
    sizes: Vec<f64>,
    total: f64,
    section: usize,
    loaded: Option<usize>,
    blocks: Vec<Block>,
    laid: Option<Laid>,
    key: Option<(Size, Settings)>,
    geometry: Geometry,
    view: Size,
    page: usize,
    scroll: f64,
    anchor: Anchor,
    images: HashMap<String, Option<ImageBrush>>,
}

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
    pub fn open(path: &Path, target: Target) -> Option<Session> {
        let book = Book::open(path)?;
        if book.sections.is_empty() {
            return None;
        }
        let sizes: Vec<f64> = (0..book.sections.len()).map(|i| book.size(i) as f64).collect();
        let total = sizes.iter().sum::<f64>().max(1.0);
        let geometry = Geometry { columns: 1, column_width: 1.0, gap: 0.0, left: 0.0, top: 0.0, height: 1.0, size: 1.0 };
        let mut session = Session {
            book,
            sizes,
            total,
            section: 0,
            loaded: None,
            blocks: Vec::new(),
            laid: None,
            key: None,
            geometry,
            view: Size::ZERO,
            page: 0,
            scroll: 0.0,
            anchor: Anchor::Start,
            images: HashMap::new(),
        };
        session.go(target);
        Some(session)
    }

    pub fn go(&mut self, target: Target) {
        match target {
            Target::Start => self.enter(0, Anchor::Start),
            Target::Cfi(source) => match self.book.resolve(&source) {
                Some((index, start, _)) => {
                    self.enter(index, Anchor::Start);
                    if let Some((block, byte)) = self.locate(start) {
                        self.anchor = Anchor::At(block, byte);
                        self.place();
                    }
                }
                None => self.enter(0, Anchor::Start),
            },
            Target::Fraction(fraction) => {
                let goal = fraction.clamp(0.0, 1.0) * self.total;
                let mut before = 0.0;
                let last = self.sizes.len() - 1;
                for (index, &size) in self.sizes.iter().enumerate() {
                    if before + size > goal || index == last {
                        let within = if size > 0.0 { ((goal - before) / size).clamp(0.0, 1.0) } else { 0.0 };
                        self.enter(index, Anchor::Fraction(within));
                        return;
                    }
                    before += size;
                }
            }
        }
    }

    fn enter(&mut self, index: usize, anchor: Anchor) {
        self.section = index;
        if self.loaded != Some(index) {
            self.loaded = Some(index);
            self.blocks = self.book.document(index).map(flow::blocks).unwrap_or_default();
            self.images.clear();
            self.laid = None;
        }
        self.anchor = anchor;
        self.place();
    }

    fn locate(&self, point: Point) -> Option<(usize, usize)> {
        let dom = self.book.document(self.section)?;
        if let Point::At(node, offset) = point
            && dom.text_of(node).is_some()
        {
            for (index, block) in self.blocks.iter().enumerate() {
                if let Block::Text { text, origins, .. } = block
                    && origins.iter().any(|o| o.node == node)
                {
                    return flow::byte(text, origins, node, offset).map(|byte| (index, byte));
                }
            }
        }
        let (enter, exit) = cfi::order(dom);
        let target = match point {
            Point::At(node, _) if dom.text_of(node).is_some() => enter[node as usize],
            Point::At(node, offset) => dom.node(node).children.get(offset as usize).map_or(exit[node as usize], |&c| enter[c as usize]),
            Point::Before(node) => enter[node as usize],
            Point::After(node) => exit[node as usize],
        };
        for (index, block) in self.blocks.iter().enumerate() {
            match block {
                Block::Text { origins, .. } => {
                    if let Some(origin) = origins.iter().find(|o| enter[o.node as usize] >= target) {
                        return Some((index, origin.byte));
                    }
                }
                Block::Image { node, .. } if enter[*node as usize] >= target => return Some((index, 0)),
                _ => {}
            }
        }
        Some((self.blocks.len().saturating_sub(1), 0))
    }

    pub fn needs_layout(&self) -> bool {
        self.laid.is_none()
    }

    pub fn ensure(&mut self, view: Size, settings: &Settings, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>) {
        let key = (view, settings.clone());
        if self.laid.is_some() && self.key.as_ref() == Some(&key) {
            return;
        }
        if self.laid.is_some() {
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

    fn scrolled(&self) -> bool {
        self.key.as_ref().is_some_and(|(_, s)| s.scrolled)
    }

    fn pages(&self) -> usize {
        let Some(laid) = &self.laid else { return 1 };
        if self.scrolled() { 1 } else { laid.columns.len().div_ceil(self.geometry.columns).max(1) }
    }

    fn max_scroll(&self) -> f64 {
        self.laid.as_ref().map_or(0.0, |l| (l.height - self.view.height + 2.0 * self.padding()).max(0.0))
    }

    fn padding(&self) -> f64 {
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

    fn place(&mut self) {
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
    }

    fn current(&self) -> Anchor {
        self.start().map_or(Anchor::Start, |(block, byte)| Anchor::At(block, byte))
    }

    fn start(&self) -> Option<(usize, usize)> {
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

    fn end(&self) -> Option<(usize, usize)> {
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

    fn point(&self, block: usize, byte: usize, forward: bool) -> Option<(u32, u32)> {
        let text_at = |index: usize, byte: Option<usize>| match &self.blocks[index] {
            Block::Text { text, origins, .. } => flow::position(text, origins, byte.unwrap_or(if forward { 0 } else { text.len() })),
            _ => None,
        };
        if let Some(found) = text_at(block, Some(byte)) {
            return Some(found);
        }
        if forward {
            (block + 1..self.blocks.len()).find_map(|i| text_at(i, None)).or_else(|| (0..block).rev().find_map(|i| text_at(i, Some(usize::MAX))))
        } else {
            (0..block).rev().find_map(|i| text_at(i, None)).or_else(|| (block + 1..self.blocks.len()).find_map(|i| text_at(i, Some(0))))
        }
    }

    pub fn location(&self) -> Option<(String, f64)> {
        let (sb, sy) = self.start()?;
        let (eb, ey) = self.end().unwrap_or((sb, sy));
        let start = self.point(sb, sy, true)?;
        let end = self.point(eb, ey, false).unwrap_or(start);
        let cfi = self.book.cfi(self.section, start, end)?;
        let within = if self.scrolled() {
            let max = self.max_scroll();
            if max > 0.0 { self.scroll / max } else { 0.0 }
        } else {
            self.page as f64 / self.pages() as f64
        };
        let before: f64 = self.sizes[..self.section].iter().sum();
        Some((cfi, ((before + within * self.sizes[self.section]) / self.total).clamp(0.0, 1.0)))
    }

    pub fn next(&mut self) -> bool {
        if self.scrolled() {
            let max = self.max_scroll();
            if self.scroll < max - 0.5 {
                self.scroll = (self.scroll + self.view.height - 2.0 * self.padding()).min(max);
                return true;
            }
        } else if self.page + 1 < self.pages() {
            self.page += 1;
            return true;
        }
        if self.section + 1 < self.sizes.len() {
            self.enter(self.section + 1, Anchor::Start);
            return true;
        }
        false
    }

    pub fn prev(&mut self) -> bool {
        if self.scrolled() {
            if self.scroll > 0.5 {
                self.scroll = (self.scroll - (self.view.height - 2.0 * self.padding())).max(0.0);
                return true;
            }
        } else if self.page > 0 {
            self.page -= 1;
            return true;
        }
        if self.section > 0 {
            self.enter(self.section - 1, Anchor::End);
            return true;
        }
        false
    }

    pub fn scroll_by(&mut self, delta: f64) -> bool {
        if !self.scrolled() {
            return false;
        }
        let max = self.max_scroll();
        let next = (self.scroll + delta).clamp(0.0, max);
        if (next - self.scroll).abs() < 0.01 {
            return false;
        }
        self.scroll = next;
        true
    }

    pub fn visible(&self) -> Vec<(Rect, f64, &Column)> {
        let Some(laid) = &self.laid else { return Vec::new() };
        if self.scrolled() {
            let Some(column) = laid.columns.first() else { return Vec::new() };
            let rect = Rect::new(self.geometry.left, 0.0, self.geometry.left + self.geometry.column_width, self.view.height);
            return vec![(rect, self.padding() - self.scroll, column)];
        }
        let first = self.page * self.geometry.columns;
        (0..self.geometry.columns)
            .filter_map(|i| laid.columns.get(first + i).map(|c| (self.geometry.column_rect(i), self.geometry.top, c)))
            .collect()
    }

    pub fn items(&self) -> &[Item] {
        self.laid.as_ref().map_or(&[], |l| &l.items)
    }

    pub fn section(&self) -> usize {
        self.section
    }

    pub fn hit(&self, p: Pos, word: bool) -> Option<(Spot, Spot)> {
        let laid = self.laid.as_ref()?;
        let visible = self.visible();
        let (rect, y0, column) = visible.iter().min_by(|a, b| {
            let d = |r: &Rect| if p.x < r.x0 { r.x0 - p.x } else if p.x > r.x1 { p.x - r.x1 } else { 0.0 };
            d(&a.0).total_cmp(&d(&b.0))
        })?;
        let piece = column
            .pieces
            .iter()
            .find(|piece| p.y < y0 + piece.y + (piece.to - piece.from))
            .or_else(|| column.pieces.last())?;
        let spot = |byte| Spot { block: piece.item, byte };
        match &laid.items[piece.item] {
            Item::Text { layout, x, breaks, .. } => {
                let local_x = (p.x - rect.x0 - x) as f32;
                let local_y = ((p.y - y0 - piece.y).max(0.0) + piece.from).min(piece.to - 0.5) as f32;
                if word {
                    let range = Selection::word_from_point(layout, local_x, local_y).text_range();
                    return Some((spot(hyphen::to_text(breaks, range.start)), spot(hyphen::to_text(breaks, range.end))));
                }
                let at = hyphen::to_text(breaks, Selection::from_point(layout, local_x, local_y).focus().index());
                Some((spot(at), spot(at)))
            }
            _ => Some((spot(0), spot(0))),
        }
    }

    fn block_range(&self, block: usize, from: Spot, to: Spot) -> Option<(usize, usize)> {
        let Block::Text { text, .. } = self.blocks.get(block)? else { return None };
        let start = if block == from.block { from.byte } else { 0 };
        let end = if block == to.block { to.byte } else { text.len() };
        (start < end).then_some((start, end))
    }

    pub fn rects(&self, from: Spot, to: Spot) -> Vec<Rect> {
        let Some(laid) = &self.laid else { return Vec::new() };
        let mut out = Vec::new();
        for (rect, y0, column) in self.visible() {
            for piece in &column.pieces {
                if piece.item < from.block || piece.item > to.block {
                    continue;
                }
                let Some((start, end)) = self.block_range(piece.item, from, to) else { continue };
                let Item::Text { layout, x, breaks, .. } = &laid.items[piece.item] else { continue };
                let anchor = Cursor::from_byte_index(layout, hyphen::to_layout(breaks, start), Affinity::Downstream);
                let focus = Cursor::from_byte_index(layout, hyphen::to_layout(breaks, end), Affinity::Upstream);
                let mut lines: Vec<(usize, f64, f64)> = Vec::new();
                for (b, line) in Selection::new(anchor, focus).geometry(layout) {
                    match lines.iter_mut().find(|(l, ..)| *l == line) {
                        Some((_, x0, x1)) => (*x0, *x1) = (x0.min(b.x0), x1.max(b.x1)),
                        None => lines.push((line, b.x0, b.x1)),
                    }
                }
                let dy = y0 + piece.y - piece.from;
                for (line, x0, x1) in lines {
                    let Some(metrics) = layout.get(line).map(|l| *l.metrics()) else { continue };
                    let (top, bottom) = (f64::from(metrics.min_coord), f64::from(metrics.max_coord));
                    if bottom <= piece.from + 0.5 || top >= piece.to - 0.5 {
                        continue;
                    }
                    out.push(Rect::new(rect.x0 + x + x0, top + dy, rect.x0 + x + x1, bottom + dy));
                }
            }
        }
        out
    }

    pub fn text(&self, from: Spot, to: Spot) -> String {
        let mut parts = Vec::new();
        for block in from.block..=to.block.min(self.blocks.len().saturating_sub(1)) {
            if let (Some((start, end)), Block::Text { text, .. }) = (self.block_range(block, from, to), &self.blocks[block]) {
                parts.push(&text[start..end]);
            }
        }
        parts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub fn range_cfi(&self, from: Spot, to: Spot) -> Option<String> {
        let start = self.point(from.block, from.byte, true)?;
        let end = self.point(to.block, to.byte, false).unwrap_or(start);
        self.book.cfi(self.section, start, end)
    }

    pub fn resolve(&self, cfi: &str) -> Option<(Spot, Spot)> {
        let (index, start, end) = self.book.resolve(cfi)?;
        if index != self.section {
            return None;
        }
        let (sb, sy) = self.locate(start)?;
        let (eb, ey) = self.locate(end)?;
        Some((Spot { block: sb, byte: sy }, Spot { block: eb, byte: ey }))
    }
}
