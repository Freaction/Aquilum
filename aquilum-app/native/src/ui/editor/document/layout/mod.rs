mod cache;
mod measure;
mod place;

use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::{Document, text};

impl Document {
    pub fn layout(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, compose: Option<(usize, &str)>, selection: Option<Range<usize>>) {
        if let Some(fonts) = self.markdown.as_mut()
            && fonts.ch == 0.0
        {
            text::measure(fcx, lcx, styles, fonts);
        }
        let selection = selection.as_ref();
        self.enter(selection);
        let rebreak = self.width != width;
        self.width = width;
        self.rebroke |= rebreak;
        if let Some(resolver) = self.resolver.clone() {
            let (unresolved, stamp) = resolver.unresolved();
            let relink = stamp != self.unresolved.1;
            let stale: Vec<usize> = self.laid.iter().filter(|(_, laid)| relink && laid.decor.links.iter().any(|l| l.wiki) || laid.decor.stale(&*resolver)).map(|(i, _)| i).collect();
            for &i in &stale {
                self.laid.remove(i);
            }
            if let (Some(&first), Some(&last)) = (stale.iter().min(), stale.iter().max()) {
                self.mark(first..last + 1);
            }
            self.unresolved = (unresolved, stamp);
        }
        if rebreak {
            self.reguess();
        }
        let mut wanted = Vec::new();
        let mut rebreak = rebreak;
        let pinned = self.pinned();
        let mut dirty = self.dirty.take();
        loop {
            let stale = dirty.take();
            let built = pinned.iter().copied().chain(self.laid.iter().map(|(i, _)| i)).map(|i| i..i + 1);
            let mut spans: Vec<Range<usize>> = built.chain([self.window()]).chain(stale.clone()).collect();
            if !self.placed {
                spans = vec![0..self.lines.count()];
            }
            spans.sort_by_key(|r| r.start);
            let mut merged: Vec<Range<usize>> = Vec::new();
            for span in spans {
                match merged.last_mut() {
                    Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
                    _ => merged.push(span),
                }
            }
            let measured = aq_trace::zone("вёрстка: высоты абзацев");
            let changed = self.measure(fcx, lcx, styles, compose, selection, rebreak, &merged, &pinned, stale.as_ref(), &mut wanted) || rebreak;
            drop(measured);
            rebreak = false;
            let placed = aq_trace::zone("вёрстка: расстановка");
            for span in &merged {
                self.place(span.start, span.end, fcx, lcx, styles, width, selection);
            }
            drop(placed);
            if !(changed | self.aligned()) {
                break;
            }
        }
        if let Some((at, top)) = self.anchor {
            let now = self.top(self.at_offset(at));
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
