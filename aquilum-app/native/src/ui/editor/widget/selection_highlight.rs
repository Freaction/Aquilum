use std::ops::{Range, RangeInclusive};
use std::time::Instant;

use masonry::imaging::Painter;
use masonry::peniko::Color;

use aquilum_core::text::matcher::Matcher;

use super::TextEditor;
use super::matching::{HOVER, SELECTION};
use crate::ui::theme;

impl TextEditor {
    pub(super) fn shown_bytes(&self, shown: &RangeInclusive<usize>) -> Range<usize> {
        self.doc.paragraph(*shown.start()).start..self.doc.paragraph(*shown.end()).end().min(self.doc.end())
    }

    fn paint_visible_matches(&self, painter: &mut Painter<'_>, shown: RangeInclusive<usize>, matcher: &Matcher, skip: Option<&Range<usize>>, color: Color) {
        let visible = self.shown_bytes(&shown);
        matcher.each(self.doc.slice(visible.clone()), |found| {
            let occurrence = visible.start + found.start..visible.start + found.end;
            if skip != Some(&occurrence) {
                self.paint_range(painter, shown.clone(), occurrence, color);
            }
            true
        });
    }

    pub(super) fn paint_selection_matches(&self, painter: &mut Painter<'_>, shown: RangeInclusive<usize>, range: &Range<usize>) {
        if !self.smart.matches || range.len() < 2 || range.len() > 64 {
            return;
        }
        let selected = &self.doc.copy(range.clone());
        if selected.contains('\n') || selected.trim().is_empty() {
            return;
        }
        let color = theme::current().search_match_bg.multiply_alpha(0.35);
        self.paint_visible_matches(painter, shown, &Matcher::new(selected, SELECTION), Some(range), color);
    }

    pub(super) fn paint_hover_terms(&self, painter: &mut Painter<'_>, shown: RangeInclusive<usize>) {
        let color = theme::current().search_match_bg.multiply_alpha(0.5);
        self.paint_visible_matches(painter, shown, &Matcher::any(&self.hover_terms, HOVER), None, color);
    }

    pub(super) fn paint_external_reveal(&self, painter: &mut Painter<'_>, shown: RangeInclusive<usize>) {
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
