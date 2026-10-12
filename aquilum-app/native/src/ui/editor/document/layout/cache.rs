use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::super::Document;

impl Document {
    pub fn invalidate(&mut self, offset: usize) {
        let i = self.at_offset(offset);
        self.paragraphs[i].forget();
        self.mark(i..i + 1);
    }

    pub fn invalidate_all(&mut self) {
        for p in &mut self.paragraphs {
            p.forget();
        }
        self.mark(0..self.paragraphs.len());
    }

    pub fn fonts_changed(&mut self) {
        if let Some(fonts) = self.markdown.as_mut() {
            fonts.ch = 0.0;
        }
        for p in &mut self.paragraphs {
            p.forget();
            p.embed = None;
        }
        self.mark(0..self.paragraphs.len());
    }

    pub fn measured(&self) -> bool {
        self.placed && self.dirty.is_none()
    }

    pub(super) fn pinned(&self) -> Vec<usize> {
        self.pins.iter().map(|&offset| self.at_offset(offset)).collect()
    }

    pub(super) fn evict(&mut self, pinned: &[usize]) {
        let keep = self.keep;
        let window = self.window();
        let old = std::mem::replace(&mut self.laid, window);
        let held = std::mem::replace(&mut self.held, pinned.to_vec());
        for i in old.chain(held) {
            if let Some(p) = self.paragraphs.get_mut(i)
                && !p.near(keep)
                && !pinned.contains(&i)
            {
                p.unbuild();
            }
        }
    }

    pub fn ensure(&mut self, shown: std::ops::RangeInclusive<usize>, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, caret: Option<usize>) {
        let pinned = self.pinned();
        let old = std::mem::replace(&mut self.shown, shown.clone());
        for i in old.filter(|i| !shown.contains(i) && !pinned.contains(i)) {
            if let Some(p) = self.paragraphs.get_mut(i) {
                p.unbuild();
            }
        }
        let width = self.width;
        for p in &mut self.paragraphs[shown] {
            if p.layout.is_none() && p.measured.is_some() {
                let local = self.markdown.as_ref().and_then(|_| p.caret(caret));
                p.build(&self.text, fcx, lcx, styles, width, None, self.markdown.as_ref().map(|fonts| (fonts, local, &*self.unresolved.0)));
            }
        }
    }
}
