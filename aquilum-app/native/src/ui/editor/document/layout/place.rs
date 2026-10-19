use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::super::super::highlight::Lang;
use super::super::super::markdown::{Block, Reveal};
use super::super::{Document, Fold};

impl Document {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn place(&mut self, from: usize, until: usize, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, selection: Option<&Range<usize>>) {
        let all = !self.placed;
        let len = self.lines.count();
        let (mut i, until) = if all { (0, len) } else { (from.min(len.saturating_sub(1)), until.min(len)) };
        while i > 0 && (self.heights.fold(i) == Fold::Member || self.inside(i) && matches!(self.blocks.get(i).1, Block::Code { first: false, lang: Some(Lang::Math), .. })) {
            i -= 1;
        }
        let mut top = self.top(i);
        let keep = self.keep;
        let put = |doc: &mut Document, k: usize, height: f64, fold: Fold| {
            if all {
                doc.heights.assign(k, height, fold);
            } else {
                doc.heights.set(k, height, fold);
            }
        };
        while i < until {
            if let Some((end, reveal)) = self.run_end(i) {
                let span = self.start(i)..self.base + self.lines.end(end);
                let open = reveal.opened(selection, &span) || reveal == Reveal::Above && self.source == Some(span.start);
                let held = self.pins.iter().any(|&pin| span.start <= pin && pin <= span.end);
                let known = match self.heights.fold(i) {
                    Fold::Head => Some(self.heights.get(i)),
                    Fold::Above => Some(self.heights.get(i) - self.heights.text(i)),
                    _ => None,
                };
                let guess = known.unwrap_or_else(|| (i..=end).map(|k| self.heights.text(k)).sum());
                let far = !held && (top + guess < keep.0 || top > keep.1);
                let built = if far {
                    self.embeds.remove(i);
                    Some(guess)
                } else {
                    (!(open && reveal == Reveal::Block)).then(|| self.embed(i, end, fcx, lcx, styles, width)).flatten()
                };
                if let Some(height) = built {
                    let beside = match reveal {
                        Reveal::Above if open => Some((i, Fold::Above)),
                        Reveal::Below if open => Some((end, Fold::Below)),
                        _ => None,
                    };
                    if let Some((at, fold)) = beside {
                        for k in i..at {
                            let text = self.heights.text(k);
                            put(self, k, text, Fold::None);
                            top += text;
                        }
                        let total = height + self.heights.text(at);
                        put(self, at, total, fold);
                        top += total;
                        i = at + 1;
                        continue;
                    }
                    put(self, i, height, Fold::Head);
                    for k in i + 1..=end {
                        put(self, k, 0.0, Fold::Member);
                    }
                    top += height;
                    i = end + 1;
                    continue;
                }
            }
            let height = self.heights.text(i);
            put(self, i, height, Fold::None);
            top += height;
            i += 1;
        }
        if all {
            self.heights.rebuild();
        }
        self.placed = true;
    }
}
