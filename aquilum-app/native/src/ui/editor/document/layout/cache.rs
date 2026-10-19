use std::ops::{Range, RangeInclusive};

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::super::Document;
use crate::ui::editor::markdown::{Block, Callout, Part, Reveal};

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

    pub fn code_run(&self, i: usize) -> (usize, usize) {
        let block = |j: usize| &self.blocks.get(j).1;
        self.around(i, |j| block(j).is_code(), |j| matches!(block(j), Block::Code { first: true, .. }))
    }

    pub(super) fn enter(&mut self, selection: Option<&Range<usize>>) {
        let (base, end) = self.loaded_range();
        let (frame, block) = (|j: usize| &self.blocks.get(j).0, |j: usize| &self.blocks.get(j).1);
        let header = |j: usize| frame(j).panels == 1 && matches!(block(j), Block::Callout { kind: Callout::Typed(_), .. });
        let unit = |i: usize| match (frame(i).panels > 0, block(i)) {
            (true, _) if Part::Callout.reveal() == Reveal::Block => self.around(i, |j| frame(j).panels > 0, header),
            (false, Block::Code { .. }) if Part::Fence.reveal() == Reveal::Block => self.code_run(i),
            (false, Block::Heading { marker: 0, .. }) if Part::Setext.reveal() == Reveal::Block => (i, (i + 1).min(self.lines.count() - 1)),
            (false, Block::Underline) if Part::Setext.reveal() == Reveal::Block => (i.saturating_sub(1), i),
            _ => (i, i),
        };
        let entered = selection.filter(|s| s.start <= end && base <= s.end).map(|s| {
            let (first, last) = (unit(self.at_offset(s.start.max(base))).0, unit(self.at_offset(s.end.min(end))).1);
            (self.start(first), self.start(last))
        });
        if entered == self.entered {
            return;
        }
        let mut stale = Vec::new();
        for (from, to) in [(self.entered, entered), (entered, self.entered)] {
            let Some((first, last)) = from else { continue };
            for i in self.at_offset(first)..=self.at_offset(last) {
                let kept = to.is_some_and(|(a, b)| (a..=b).contains(&self.start(i)));
                if !kept && (frame(i).panels > 0 || matches!(block(i), Block::Code { fence: true, .. } | Block::Underline)) {
                    stale.push(i);
                }
            }
        }
        for i in stale {
            self.laid.remove(i);
        }
        self.entered = entered;
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

    pub fn ensure(&mut self, shown: RangeInclusive<usize>, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, selection: Option<&Range<usize>>) {
        for i in shown {
            if self.laid.get(i).is_none() {
                let laid = self.lay(i, fcx, lcx, styles, None, selection);
                self.heights.set_text(i, laid.height());
                self.laid.insert(i, laid);
            }
        }
    }
}
