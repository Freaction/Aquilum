use std::ops::Range;
use std::time::Instant;

use masonry::imaging::Painter;

use super::TextEditor;
use crate::ui::editor::document::Paragraph;
use crate::ui::theme;

impl TextEditor {
    pub(super) fn paint_selection_matches(&self, painter: &mut Painter<'_>, shown: &[Paragraph], range: &Range<usize>) {
        if range.len() < 2 || range.len() > 64 {
            return;
        }
        let doc_text = self.doc.text();
        let selected = &doc_text[range.clone()];
        if selected.contains('\n') || selected.trim().is_empty() {
            return;
        }
        let tm = theme::current();
        let match_color = tm.search_match_bg.multiply_alpha(0.35);
        for (idx, _) in doc_text.match_indices(selected) {
            let occ = idx..idx + selected.len();
            if occ != *range {
                self.paint_range(painter, shown, occ, match_color);
            }
        }
    }

    pub(super) fn paint_hover_terms(&self, painter: &mut Painter<'_>, shown: &[Paragraph]) {
        let color = theme::current().search_match_bg.multiply_alpha(0.5);
        for mark in &self.hover_marks {
            self.paint_range(painter, shown, mark.clone(), color);
        }
    }

    pub(super) fn find_hover_marks(&mut self) {
        let text = self.doc.text();
        self.hover_marks = self.hover_terms.iter().flat_map(|term| super::find::find_all(text, term)).collect();
    }

    pub(super) fn paint_external_reveal(&self, painter: &mut Painter<'_>, shown: &[Paragraph]) {
        let Some((range, start)) = &self.external_reveal else { return };
        let elapsed = start.elapsed().as_secs_f32();
        if elapsed < 0.9 {
            let alpha = (1.0 - elapsed / 0.9).clamp(0.0, 1.0);
            let tm = theme::current();
            let color = tm.bg_accent.multiply_alpha(alpha * 0.4);
            self.paint_range(painter, shown, range.clone(), color);
        }
    }

    pub fn set_hover_terms(this: &mut masonry::core::WidgetMut<'_, Self>, terms: Vec<String>) {
        if this.widget.hover_terms != terms {
            this.widget.hover_terms = terms;
            this.widget.find_hover_marks();
            this.ctx.request_paint_only();
        }
    }

    pub fn reveal_external_insert(this: &mut masonry::core::WidgetMut<'_, Self>, range: Range<usize>) {
        if !crate::ui::motion::on() {
            return;
        }
        this.widget.external_reveal = Some((range, Instant::now()));
        this.ctx.request_anim_frame();
        this.ctx.request_paint_only();
    }
}
