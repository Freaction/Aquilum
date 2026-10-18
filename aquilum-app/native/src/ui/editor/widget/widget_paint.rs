use std::ops::{Range, RangeInclusive};
use masonry::core::{PaintCtx, PropertiesRef, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, RoundedRect};
use masonry::parley::{Affinity, BoundingBox, Cursor};
use masonry::properties::{ContentColor, SelectionColor};
use super::Fold;
use crate::ui::editor::markdown;
use super::TextEditor;
use crate::ui::theme;
use crate::ui::tokens::size;

impl TextEditor {
    pub(super) fn do_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let _zone = aq_trace::zone("редактор: отрисовка текста");
        let (top, bottom) = super::visible(ctx.window_transform(), ctx.window_size());
        let (margin, keep) = (bottom - top, self.doc.keep);
        self.painted = ((top - margin).max(keep.0), (bottom + margin).min(keep.1));
        let (first, last) = (self.doc.at_y(self.painted.0), self.doc.at_y(self.painted.1));
        if self.laid_out() {
            let (fcx, lcx) = ctx.text_contexts();
            self.doc.ensure(first..=last, fcx, lcx, &self.styles, self.focused.then_some(self.focus));
        }
        let shown = first..=last;
        let texts: Vec<usize> = shown.clone().filter(|&i| {
            let p = self.doc.paragraph(i);
            p.laid().is_some() && !matches!((p.fold, &p.embed), (Fold::Member, _) | (Fold::Head, Some(_)))
        }).collect();
        let color = props.get::<ContentColor>(ctx.property_cache()).color;
        let tm = theme::current();
        let palette = markdown::palette(&tm, color);
        let markdown = self.doc.markdown.is_some();
        if markdown {
            self.clips = self.paint_blocks(painter, first, last, &palette);
            for &i in &texts {
                self.paint_backdrop(painter, i);
            }
        }
        let range = self.selection();
        if !range.is_empty() && ctx.is_focus_target() {
            let sel_color = props.get::<SelectionColor>(ctx.property_cache()).color;
            self.paint_range(painter, shown.clone(), range.clone(), sel_color);
            self.paint_selection_matches(painter, shown.clone(), &range);
        }
        self.paint_extra_selections(painter, shown.clone(), props.get::<SelectionColor>(ctx.property_cache()).color);
        self.paint_hover_terms(painter, shown.clone());
        self.paint_external_reveal(painter, shown.clone());
        for found in self.find_marks(&shown) {
            self.paint_range(painter, shown.clone(), found.clone(), tm.search_match_bg);
        }
        for (i, top, _) in self.doc.placed(shown) {
            let p = self.doc.paragraph(i);
            let (Fold::Head | Fold::Above, Some(embed)) = (p.fold, &p.embed) else { continue };
            let mut overlay = match &self.table {
                Some(session) if session.start == p.start => session.overlay(self.hovered_table == Some(p.start)),
                _ => super::super::table::Overlay { hovered: self.hovered_table == Some(p.start), scroll_x: self.table_scroll_x(p.start), ..Default::default() },
            };
            overlay.search = self.block_search(p.start..self.doc.folded_end(i));
            let media = self.media_overlay(p.start);
            if media.video.is_none() {
                let hover = self.hovered_link.filter(|(start, _)| *start == p.start).map(|(_, hit)| hit);
                embed.view.paint(painter, self.embed_x(i), top, &palette, &overlay, &media, hover);
            }
            self.paint_link_hover(painter, i);
        }
        for i in texts {
            if markdown {
                self.paint_marks(painter, i, &palette);
            }
            self.clipped(painter, i, |painter| render_text(painter, Affine::translate(self.shift() + self.doc.origin(i)), self.doc.paragraph(i).layout(), &palette, true));
        }
    }

    pub(super) fn do_post_paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if !self.videos.is_empty() && self.laid_out() {
            self.paint_videos(painter);
        }
        if !(ctx.is_focus_target() && ctx.is_window_focused() && self.laid_out()) {
            return;
        }
        if self.table_active() {
            self.paint_table_caret(painter);
            return;
        }
        if !self.caret_visible {
            return;
        }
        painter.fill(RoundedRect::from_rect(self.caret(), size::CARET_RADIUS), theme::current().caret_color).draw();
        self.paint_extra_carets(painter, theme::current().caret_color);
    }

    pub(super) fn paint_range(&self, painter: &mut Painter<'_>, shown: RangeInclusive<usize>, range: Range<usize>, color: masonry::peniko::Color) {
        for i in shown {
            let p = self.doc.paragraph(i);
            if matches!(p.fold, Fold::Head | Fold::Member) || p.laid().is_none() || p.start > range.end || p.end() < range.start {
                continue;
            }
            let origin = self.doc.origin(i);
            let layout = p.layout();
            let a = range.start.max(p.start) - p.start;
            let b = range.end.min(p.end()) - p.start;
            self.clipped(painter, i, |painter| {
                for (bounds, _) in markdown::range_rects(layout, a, b) {
                    painter.fill(super::rect(BoundingBox::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1), 0.0) + origin + self.shift(), color).draw();
                }
            });
            if range.end > p.end() && b == p.len as usize {
                let end = Cursor::from_byte_index(layout, p.len as usize, Affinity::Upstream).geometry(layout, 1.0);
                painter.fill(super::rect(BoundingBox::new(end.x0, end.y0, end.x0 + 8.0, end.y1), 0.0) + origin + self.shift(), color).draw();
            }
        }
    }

    pub(super) fn clipped(&self, painter: &mut Painter<'_>, i: usize, paint: impl FnOnce(&mut Painter<'_>)) {
        let clip = self.clips.iter().find(|(lines, _)| lines.contains(&i)).map(|(_, clip)| *clip);
        if let Some(clip) = clip {
            painter.push_fill_clip(clip);
        }
        paint(painter);
        if clip.is_some() {
            painter.pop_clip();
        }
    }
}
