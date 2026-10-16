use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::super::Document;

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
        let (heights, keep) = (&self.heights, self.keep);
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
