use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{Alignment, AlignmentOptions, FontContext, LayoutContext};

use crate::ui::editor::markdown::Block;
use super::super::Document;
use super::super::laid::PLAIN;

impl Document {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn measure(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, compose: Option<(usize, &str)>, caret: Option<usize>, rebreak: bool, spans: &[Range<usize>], pinned: &[usize], stale: Option<&Range<usize>>, wanted: &mut Vec<String>) -> bool {
        let keep = self.kept();
        let mut line = self.line_height();
        let mut changed = false;
        let (len, mut next) = (self.lines.count(), 0);
        let indices = spans.iter().flat_map(|r| r.clone()).filter(|&i| {
            let fresh = i >= next && i < len;
            next = next.max(i + 1);
            fresh
        });
        for i in indices {
            let near = self.placed && self.heights.near(i, keep) || pinned.contains(&i);
            let range = self.span(i);
            let composing = compose.filter(|(at, _)| range.contains(at) || *at == range.end);
            let moved = self.markdown.is_some() && self.local_caret(i, caret) != self.laid.get(i).map_or(&PLAIN, |laid| &laid.decor).caret;
            let current = composing.is_none() && !moved;
            match self.laid.get_mut(i).filter(|_| current) {
                Some(laid) => {
                    if rebreak {
                        laid.layout.break_all_lines(self.width.map(|w| (w - laid.decor.inset as f32).max(0.0)));
                        laid.layout.align(Alignment::Start, AlignmentOptions::default());
                        self.heights.set_text(i, laid.height());
                    }
                }
                None if current && !near => {
                    if stale.is_some_and(|stale| stale.contains(&i)) {
                        let height = self.estimate(i, line);
                        changed |= self.heights.set_text(i, height);
                    }
                }
                None => {
                    let laid = self.lay(i, fcx, lcx, styles, composing, caret);
                    changed |= self.heights.set_text(i, laid.height());
                    if *self.blocks.get(i) == Block::Text && laid.layout.len() == 1 {
                        line = laid.height();
                    }
                    wanted.extend(laid.decor.links.iter().filter(|l| l.wiki).map(|l| l.target.trim().to_owned()));
                    if near {
                        self.laid.insert(i, laid);
                    }
                }
            }
        }
        self.line = line;
        changed
    }
}
