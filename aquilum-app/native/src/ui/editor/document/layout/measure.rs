use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{Alignment, AlignmentOptions, FontContext, LayoutContext};

use crate::ui::editor::markdown::Block;

use super::super::Document;
use super::super::paragraph::PLAIN;

impl Document {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn measure(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, compose: Option<(usize, &str)>, caret: Option<usize>, rebreak: bool, spans: &[Range<usize>], pinned: &[usize], wanted: &mut Vec<String>) -> bool {
        let keep = self.keep;
        let ch = self.markdown.as_ref().map_or(0.0, |fonts| f64::from(fonts.ch));
        let mut line = if self.line > 0.0 { self.line } else { self.markdown.as_ref().map_or(20.0, |fonts| f64::from(fonts.size) * 1.6) };
        let mut changed = false;
        let (len, mut next) = (self.paragraphs.len(), 0);
        let indices = spans.iter().flat_map(|r| r.clone()).filter(|&i| {
            let fresh = i >= next && i < len;
            next = next.max(i + 1);
            fresh
        });
        for i in indices {
            let p = &mut self.paragraphs[i];
            let near = p.measured.is_some() && p.near(keep) || pinned.contains(&i);
            let composing = compose.filter(|(at, _)| p.range().contains(at) || *at == p.end());
            let local = self.markdown.as_ref().and_then(|_| p.caret(caret));
            let decor = p.decor.as_deref().unwrap_or(&PLAIN);
            let moved = self.markdown.is_some() && local != decor.caret;
            let wrap = width.map(|w| (w - decor.inset as f32).max(0.0));
            let pad = decor.pad.0 + decor.pad.1;
            match (&mut p.layout, composing) {
                (Some(layout), None) if rebreak && !moved => {
                    layout.break_all_lines(wrap);
                    layout.align(wrap, Alignment::Start, AlignmentOptions::default());
                    p.measured = Some((f64::from(layout.height()) + pad) as f32);
                }
                (Some(_), None) if !moved => {}
                (None, None) if !moved && p.exact && !rebreak => {}
                (None, None) if !moved && !near => {
                    if p.measured.is_none() {
                        changed = true;
                        p.estimate(&self.text, line, ch, wrap);
                    } else if rebreak {
                        p.exact = false;
                    }
                }
                (_, composing) => {
                    changed |= !p.exact;
                    p.build(&self.text, fcx, lcx, styles, width, composing, self.markdown.as_ref().map(|fonts| (fonts, local, &*self.unresolved.0)));
                    if p.block == Block::Text && p.layout().len() == 1 {
                        line = p.text_height();
                    }
                    wanted.extend(p.decor().links.iter().filter(|l| l.wiki).map(|l| l.target.trim().to_owned()));
                    if !near {
                        p.unbuild();
                    }
                }
            }
        }
        self.line = line;
        changed
    }
}
