use masonry::core::{ComposeCtx, CursorIcon, LayoutCtx, MeasureCtx, PropertiesMut, PropertiesRef, QueryCtx, Update, UpdateCtx};
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};
use super::TextEditor;

impl TextEditor {
    pub(super) fn do_update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                if let Some(style) = self.style {
                    for property in style() {
                        self.styles.insert(property);
                    }
                }
                self.doc.fonts_changed();
                ctx.request_layout();
            }
            Update::FocusChanged(focused) => {
                self.focused = *focused;
                if *focused {
                    self.reset_blink();
                } else {
                    self.hide_suggest();
                }
                ctx.request_layout();
                ctx.request_render();
                ctx.request_anim_frame();
            }
            _ => {}
        }
    }

    pub(super) fn do_measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::MinContent => Length::ZERO,
                LenReq::MaxContent => Length::px(self.width),
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => {
                let width = cross.map_or(self.width, Length::get);
                let (fcx, lcx) = ctx.text_contexts();
                self.ensure_layout(fcx, lcx, width);
                Length::px(self.doc.height() + self.past_end)
            }
        }
    }

    pub(super) fn do_layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let (fcx, lcx) = ctx.text_contexts();
        self.ensure_layout(fcx, lcx, size.width);
        let shift = self.doc.take_shift();
        if shift != 0.0 {
            ctx.request_scroll_by(shift);
        }
        self.reveal_caret();
        self.follow_code();
        if self.suggest.is_some() {
            self.publish_suggest(ctx.widget_id());
        }
        if self.single_line {
            ctx.set_clip_path(size.to_rect());
        }
    }

    pub(super) fn do_compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        let (top, bottom) = super::visible(ctx.window_transform(), ctx.window_size());
        if top < self.painted.0 || bottom > self.painted.1 {
            ctx.request_paint_only();
        }
        if std::mem::take(&mut self.doc.rebroke) {
            ctx.request_layout();
        }
        if self.single_line {
            return;
        }
        if self.doc.track((top, bottom)) {
            ctx.request_layout();
        }
        if self.doc.growing() {
            crate::ui::timer::wake(ctx.widget_id(), std::time::Instant::now());
        }
        self.doc.anchor_at(top.max(0.0));
        let (keep, margin) = (self.doc.keep, bottom - top);
        let cover = margin / 4.0;
        if top - cover < keep.0 && keep.0 > 0.0 || bottom + cover > keep.1 {
            self.doc.keep = (top - margin, bottom + margin);
            ctx.request_layout();
        }
    }

    pub(super) fn do_get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        self.media_cursor()
            .or_else(|| self.table_cursor().filter(|_| self.hovered_table.is_some()))
            .or(self.code_bar.map(|_| CursorIcon::Default))
            .or(self.pointer_hover.then_some(CursorIcon::Pointer))
            .unwrap_or(CursorIcon::Text)
    }
}
