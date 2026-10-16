use std::ops::Range;

use super::Document;

impl Document {
    pub fn height(&self) -> f64 {
        self.above + self.heights.total() + self.below
    }

    pub fn top(&self, index: usize) -> f64 {
        self.above + self.heights.top(index)
    }

    pub fn bottom(&self, index: usize) -> f64 {
        self.top(index) + self.heights.get(index)
    }

    pub fn origin(&self, index: usize) -> masonry::kurbo::Vec2 {
        self.paragraph(index).origin(self.top(index))
    }

    pub fn placed(&self, range: std::ops::RangeInclusive<usize>) -> impl Iterator<Item = (usize, f64, f64)> + '_ {
        let mut top = self.top(*range.start());
        range.map(move |i| {
            let (at, height) = (top, self.heights.get(i));
            top += height;
            (i, at, height)
        })
    }

    pub fn at_offset(&self, offset: usize) -> usize {
        self.lines.at(offset.saturating_sub(self.base))
    }

    pub fn at_y(&self, y: f64) -> usize {
        self.heights.at(y - self.above)
    }

    pub(super) fn kept(&self) -> (f64, f64) {
        (self.keep.0 - self.above, self.keep.1 - self.above)
    }

    pub fn take_shift(&mut self) -> f64 {
        std::mem::take(&mut self.shift)
    }

    pub fn anchor_at(&mut self, y: f64) {
        let i = self.at_y(y + 8.0);
        self.anchor = self.placed.then_some((self.start(i), self.top(i)));
    }

    pub(super) fn start(&self, line: usize) -> usize {
        self.base + self.lines.start(line)
    }

    pub(super) fn span(&self, line: usize) -> Range<usize> {
        self.start(line)..self.base + self.lines.end(line)
    }
}
