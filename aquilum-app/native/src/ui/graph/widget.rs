use std::time::Instant;

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, Size};
use masonry::layout::{LenReq, Length};

use crate::ui::text::Line;
use crate::ui::theme;
use crate::ui::tokens::size;
use super::{CAMERA_SETTLE, DRAG_THRESHOLD, DRAFT_BUDGET, GraphAction, GraphView, SETTINGS_WIDTH, Status};
use crate::i18n::t;

impl Widget for GraphView {
    type Action = GraphAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        self.pointer(ctx, event);
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let now = Instant::now();
        let seconds = interval as f64 / 1e9;
        self.scale_factor = ctx.scale_factor();
        let mut busy = self.step(seconds);
        let (fcx, lcx) = ctx.text_contexts();
        busy |= self.step_labels(seconds, fcx, lcx);
        ctx.request_paint_only();
        if let Some(at) = self.moved_at
            && now.duration_since(at) >= CAMERA_SETTLE
        {
            self.moved_at = None;
            let s = self.display.spread;
            ctx.submit_action::<GraphAction>(GraphAction::Camera(self.camera.center_x / s, self.camera.center_y / s, self.camera.scale));
        }
        if let (Some(p), None, None) = (self.pointer, &self.drag, &self.morph) {
            let node = self.node_at(p);
            busy |= self.hover(node);
        }
        let moving = busy || self.drag.as_ref().is_some_and(|d| d.travelled > DRAG_THRESHOLD);
        self.draft = moving && (self.draft || self.cost > DRAFT_BUDGET);
        if busy || self.moved_at.is_some() || std::mem::take(&mut self.animating) {
            ctx.request_anim_frame();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.labels.clear();
                self.sprites.clear();
                self.status_lines.clear();
                self.notice = [Line::default(), Line::default()];
                ctx.request_anim_frame();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(settings) = &mut self.settings {
            ctx.register_child(settings);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let resized = self.size != size;
        self.size = size;
        ctx.set_clip_path(size.to_rect());
        if self.graph.is_some() && size.width > 0.0 && size.height > 0.0 {
            if let Some((x, y, scale)) = self.stored.take() {
                let s = self.display.spread;
                self.camera.move_to(x * s, y * s, scale);
                self.fitted = true;
            } else if !self.fitted {
                self.camera.jump_to(self.bounds(), size.width, size.height);
                self.fitted = true;
            }
            if resized {
                let (fcx, lcx) = ctx.text_contexts();
                self.step_labels(0.0, fcx, lcx);
            }
        }
        if let Some(settings) = &mut self.settings {
            let show = !matches!(self.status, Status::Failed(_) | Status::NoWorkspace);
            ctx.set_stashed(settings, !show);
            self.panel = Rect::ZERO;
            if show {
                let panel = Rect::new(size.width - size::SPACE_12 - SETTINGS_WIDTH, size::SPACE_12, size.width - size::SPACE_12, (size.height - size::SPACE_12).max(size::SPACE_12));
                ctx.run_layout(settings, panel.size());
                ctx.place_child(settings, panel.origin());
                self.panel = panel;
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let bounds = ctx.content_box();
        match &self.status {
            Status::NoWorkspace => {
                painter.fill(bounds, tm.graph_backdrop).draw();
                return self.paint_notice(ctx, painter, &t("graph.noWorkspace"), None, None);
            }
            Status::Failed(error) => {
                painter.fill(bounds, tm.graph_backdrop).draw();
                let error = error.clone();
                return self.paint_notice(ctx, painter, &t("graph.notBuilt"), Some(&error), Some(&t("graph.retry")));
            }
            Status::Loading if self.graph.is_none() => {
                painter.fill(bounds, tm.graph_backdrop).draw();
                self.pill(ctx, painter, &[t("graph.loading")]);
                return;
            }
            _ => {}
        }
        self.scale_factor = ctx.scale_factor();
        let Some(graph) = self.graph.clone() else { return };
        let started = Instant::now();
        if let Some((image, scale)) = self.scene() {
            painter.draw_image(&image, Affine::scale(1.0 / scale));
        }
        self.cost = started.elapsed();
        self.paint_status(ctx, painter, &graph);
        if self.visible == 0 && graph.len() > 0 {
            self.paint_notice(ctx, painter, &t("graph.filteredOut"), None, Some(&t("graph.showAll")));
        } else {
            self.action_rect = None;
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        if self.drag.as_ref().is_some_and(|d| d.travelled > DRAG_THRESHOLD) {
            CursorIcon::Grabbing
        } else if self.hovered.is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(t("tabs.graph"));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&self.settings.iter().map(WidgetPod::id).collect::<Vec<_>>())
    }
}
