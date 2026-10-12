mod cache;
mod measure;
mod place;

use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::{Document, text};

impl Document {
    pub fn layout(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, compose: Option<(usize, &str)>, caret: Option<usize>) {
        if let Some(fonts) = self.markdown.as_mut()
            && fonts.ch == 0.0
        {
            fonts.ch = text::measure_ch(fcx, lcx, fonts);
        }
        let rebreak = self.width != width;
        self.width = width;
        self.rebroke |= rebreak;
        if let Some((unresolved, stamp)) = self.resolver.as_ref().map(|r| r.unresolved())
            && stamp != self.unresolved.1
        {
            for p in self.paragraphs.iter_mut().filter(|p| p.decor().links.iter().any(|l| l.wiki)) {
                p.forget();
            }
            self.mark(0..self.paragraphs.len());
            self.unresolved = (unresolved, stamp);
        }
        let mut wanted = Vec::new();
        let mut rebreak = rebreak;
        let pinned = self.pinned();
        let mut dirty = self.dirty.take();
        loop {
            let window = self.window();
            let held = pinned.iter().chain(&self.held).map(|&i| i..i + 1);
            let mut spans: Vec<Range<usize>> = held.chain([window, self.laid.clone()]).chain(dirty.take()).collect();
            if rebreak {
                spans = vec![0..self.paragraphs.len()];
            }
            spans.sort_by_key(|r| r.start);
            let changed = self.measure(fcx, lcx, styles, width, compose, caret, rebreak, &spans, &pinned, &mut wanted) || rebreak;
            rebreak = false;
            let (from, until) = (spans.iter().map(|r| r.start).min().unwrap_or(0), spans.iter().map(|r| r.end).max().unwrap_or(0));
            self.place(from, until, fcx, lcx, styles, width, caret);
            if !changed {
                break;
            }
        }
        if let Some((at, top)) = self.anchor {
            let now = self.paragraphs[self.at_offset(at)].top;
            if (now - top).abs() > 1.0 {
                self.shift += now - top;
            }
            self.anchor = Some((at, now));
        }
        self.evict(&pinned);
        if let Some(resolver) = &self.resolver {
            resolver.sweep();
            if !wanted.is_empty() {
                resolver.resolve(wanted);
            }
        }
    }

    fn window(&self) -> Range<usize> {
        self.at_y(self.keep.0)..self.at_y(self.keep.1) + 1
    }
}
