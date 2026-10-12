use masonry::kurbo::Rect;

use crate::reader::flow::{self, Block};
use crate::reader::page::{Column, Item};
use super::{Anchor, Session};

impl Session {
    pub(super) fn point(&self, block: usize, byte: usize, forward: bool) -> Option<(u32, u32)> {
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
}
