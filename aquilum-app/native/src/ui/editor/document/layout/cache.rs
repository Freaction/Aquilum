use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::super::Document;
use crate::ui::editor::markdown::{Block, Callout};

impl Document {
    pub fn invalidate(&mut self, offset: usize) {
        let i = self.at_offset(offset);
        self.laid.remove(i);
        self.mark(i..i + 1);
    }

    pub(in super::super) fn invalidate_all(&mut self) {
        self.laid.clear();
        self.mark(0..self.lines.count());
    }

    pub fn around(&self, i: usize, same: impl Fn(usize) -> bool, starts: impl Fn(usize) -> bool) -> (usize, usize) {
        let first = (0..=i).rev().find(|&j| starts(j) || j == 0 || !same(j - 1)).unwrap_or(i);
        let last = (i..self.lines.count()).find(|&j| j + 1 == self.lines.count() || !same(j + 1) || starts(j + 1)).unwrap_or(i);
        (first, last)
    }

    pub(super) fn enter(&mut self, caret: Option<usize>) {
        let (base, end) = self.loaded_range();
        let caret = caret.filter(|c| (base..=end).contains(c));
        let within = |(first, last): (usize, usize)| caret.is_some_and(|c| first <= c && self.at_offset(c) <= self.at_offset(last));
        if self.dirty.is_none() && self.entered.is_some_and(within) {
            return;
        }
        let (frame, block) = (|j: usize| &self.blocks.get(j).0, |j: usize| &self.blocks.get(j).1);
        let header = |j: usize| frame(j).panels == 1 && matches!(block(j), Block::Callout { kind: Callout::Typed(_), .. });
        let entered = caret.map(|c| self.at_offset(c)).and_then(|i| match (frame(i).panels > 0, block(i).is_code()) {
            (true, _) => Some(self.around(i, |j| frame(j).panels > 0, header)),
            (false, true) => Some(self.around(i, |j| block(j).is_code(), |j| matches!(block(j), Block::Code { first: true, .. }))),
            _ => None,
        });
        let entered = entered.map(|(first, last)| (self.start(first), self.start(last)));
        if entered != self.entered {
            for (first, last) in [self.entered, entered].into_iter().flatten() {
                let (first, last) = (self.at_offset(first), self.at_offset(last));
                let fences = self.blocks.get(first).1.is_code();
                for i in (first..=last).filter(|&i| !fences || i == first || i == last) {
                    self.laid.remove(i);
                }
            }
            self.entered = entered;
        }
    }

    pub fn fonts_changed(&mut self) {
        if let Some(fonts) = self.markdown.as_mut() {
            fonts.ch = 0.0;
        }
        self.embeds.clear();
        self.invalidate_all();
    }

    pub fn measured(&self) -> bool {
        self.placed && self.dirty.is_none()
    }

    pub(super) fn pinned(&self) -> Vec<usize> {
        self.pins.iter().map(|&offset| self.at_offset(offset)).collect()
    }

    pub(super) fn evict(&mut self, pinned: &[usize]) {
        let (heights, keep) = (&self.heights, self.kept());
        self.laid.retain(|i| heights.near(i, keep) || pinned.contains(&i));
    }

    pub fn ensure(&mut self, shown: std::ops::RangeInclusive<usize>, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, caret: Option<usize>) {
        for i in shown {
            if self.laid.get(i).is_none() {
                let laid = self.lay(i, fcx, lcx, styles, None, caret);
                self.heights.set_text(i, laid.height());
                self.laid.insert(i, laid);
            }
        }
    }
}
