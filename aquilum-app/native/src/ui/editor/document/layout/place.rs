use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{FontContext, LayoutContext};

use super::super::embeds::Reveal;
use super::super::{Document, Fold, Paragraph};

impl Document {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn place(&mut self, from: usize, until: usize, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, width: Option<f32>, caret: Option<usize>) {
        let mut i = if self.placed { from.min(self.paragraphs.len().saturating_sub(1)) } else { 0 };
        while i > 0 && self.paragraphs[i].fold == Fold::Member {
            i -= 1;
        }
        let mut top = i.checked_sub(1).map_or(0.0, |k| self.paragraphs[k].top + self.paragraphs[k].height);
        let keep = self.keep;
        self.placed = true;
        while i < self.paragraphs.len() {
            if i >= until && self.paragraphs[i].top == top {
                break;
            }
            if let Some((end, reveal)) = self.run_end(i) {
                let span = self.paragraphs[i].start..=self.paragraphs[end].end();
                let inside = caret.is_some_and(|c| span.contains(&c));
                let known = self.paragraphs[i].embedded.map(f64::from);
                let guess = known.unwrap_or_else(|| self.paragraphs[i..=end].iter().map(Paragraph::text_height).sum());
                let far = !inside && self.source != Some(*span.start()) && (top + guess < keep.0 || top > keep.1);
                let built = if far {
                    self.paragraphs[i].embed = None;
                    Some(guess)
                } else {
                    let built = (!(inside && reveal == Reveal::Source)).then(|| self.embed(i, end, fcx, lcx, styles, width)).flatten();
                    self.paragraphs[i].embedded = built.map(|h| h as f32);
                    built
                };
                if let Some(height) = built {
                    if reveal == Reveal::Above && self.source == Some(self.paragraphs[i].start) {
                        let p = &mut self.paragraphs[i];
                        p.fold = Fold::Above;
                        p.lift = height as f32;
                        p.top = top;
                        p.height = height + p.text_height();
                        top += p.height;
                        i += 1;
                        continue;
                    }
                    for (k, p) in self.paragraphs[i..=end].iter_mut().enumerate() {
                        p.fold = if k == 0 { Fold::Head } else { Fold::Member };
                        p.lift = 0.0;
                        p.top = if k == 0 { top } else { top + height };
                        p.height = if k == 0 { height } else { 0.0 };
                    }
                    top += height;
                    i = end + 1;
                    continue;
                }
            }
            let p = &mut self.paragraphs[i];
            p.fold = Fold::None;
            p.lift = 0.0;
            p.top = top;
            p.height = p.text_height();
            top += p.height;
            i += 1;
        }
    }
}
