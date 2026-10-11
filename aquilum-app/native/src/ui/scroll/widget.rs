use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, ComposeCtx, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerButton, PointerEvent, PointerScrollEvent, PropertiesMut,
    PropertiesRef, RegisterCtx, ScrollDelta, Update, UpdateCtx, Widget,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenReq, Length};

use super::area::ScrollArea;
use super::{LINE, Scrolled};
use crate::ui::tokens::size;

impl<W: Widget> ScrollArea<W> {
    fn wheel_delta(&self, x: f64, y: f64, shift: bool) -> f64 {
        match self.axis {
            Axis::Vertical => y,
            Axis::Horizontal if shift && x == 0.0 => y,
            Axis::Horizontal if x.abs() > y.abs() => x,
            Axis::Horizontal => 0.0,
        }
    }

    fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Scroll(PointerScrollEvent { delta, state, .. }) => {
                let before = (self.offset, self.glide.map(|g| g.to));
                let shift = state.modifiers.shift();
                match *delta {
                    ScrollDelta::LineDelta(x, y) => {
                        if self.glide_by(-self.wheel_delta(f64::from(x), f64::from(y), shift) * LINE) {
                            ctx.request_anim_frame();
                        }
                    }
                    ScrollDelta::PixelDelta(p) => {
                        let d = self.wheel_delta(p.x, p.y, shift);
                        self.jump(self.offset - d / ctx.scale_factor());
                    }
                    ScrollDelta::PageDelta(x, y) => {
                        if self.glide_by(-self.wheel_delta(f64::from(x), f64::from(y), shift) * self.view()) {
                            ctx.request_anim_frame();
                        }
                    }
                }
                if (self.offset, self.glide.map(|g| g.to)) != before {
                    ctx.request_compose();
                    ctx.request_post_paint();
                    ctx.set_handled();
                }
            }
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) && self.overflow => {
                let at = ctx.local_position(e.state.position);
                let span = self.span();
                if let Some(next) = self.bar.press(at, &span) {
                    if self.bar.dragging() {
                        self.glide = None;
                        ctx.capture_pointer();
                    } else {
                        self.jump(next);
                        ctx.request_compose();
                    }
                    ctx.request_post_paint();
                    ctx.set_handled();
                }
            }
            PointerEvent::Move(update) => {
                let at = ctx.local_position(update.current.position);
                if let Some(next) = self.bar.drag(at, &self.span()) {
                    self.offset = next;
                    ctx.request_compose();
                    ctx.request_post_paint();
                }
                let span = self.span();
                if self.bar.hover(Some(at).filter(|_| self.overflow), &span) {
                    ctx.request_post_paint();
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) if self.bar.release() => ctx.request_post_paint(),
            PointerEvent::Leave(_) if !self.bar.dragging() => {
                let span = self.span();
                if self.bar.hover(None, &span) {
                    ctx.request_post_paint();
                }
            }
            _ => {}
        }
    }

    fn measure_cross(&mut self, ctx: &mut MeasureCtx<'_>, cross: Option<Length>) -> Length {
        let content = ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), LayoutSize::NONE, Axis::Horizontal, None);
        let height = ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), LayoutSize::NONE, Axis::Vertical, Some(content));
        let overflow = cross.is_some_and(|width| content.get() > width.get() + 0.5);
        Length::px(height.get() + if overflow { size::SCROLLBAR_WIDTH } else { 0.0 })
    }
}

impl<W: Widget> Widget for ScrollArea<W> {
    type Action = Scrolled;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let before = self.offset;
        self.pointer(ctx, event);
        if self.offset != before {
            ctx.submit_action::<Scrolled>(Scrolled(self.offset));
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let Some(mut glide) = self.glide else { return };
        let running = glide.advance((interval as f64 / 1e9).min(0.05));
        self.offset = glide.position().clamp(0.0, self.max_offset());
        self.glide = running.then_some(glide);
        if running {
            ctx.request_anim_frame();
        }
        ctx.request_compose();
        ctx.request_post_paint();
        ctx.submit_action::<Scrolled>(Scrolled(self.offset));
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::ScrollBy(dy) = event
            && self.axis == Axis::Vertical
        {
            self.offset += dy;
            if let Some(glide) = &mut self.glide {
                glide.shift(*dy);
            }
            self.clamp();
            ctx.request_compose();
            ctx.request_post_paint();
        }
        if let Update::RequestPanToChild(target) = event {
            self.glide = None;
            let (start, end) = match self.axis {
                Axis::Vertical => (target.y0, target.y1),
                Axis::Horizontal => (target.x0, target.x1),
            };
            if start < self.offset {
                self.offset = start;
            } else if end > self.offset + self.view() {
                self.offset = end - self.view();
            }
            self.clamp();
            ctx.request_compose();
            ctx.request_post_paint();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        if self.axis == Axis::Horizontal && axis == Axis::Vertical {
            return self.measure_cross(ctx, cross);
        }
        match len_req {
            LenReq::MinContent => Length::ZERO,
            LenReq::FitContent(space) => space,
            LenReq::MaxContent => {
                let context = LayoutSize::maybe(axis.cross(), cross);
                ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross.filter(|_| axis == Axis::Vertical))
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        self.viewport = size;
        let axis = self.axis;
        let room = size.get_coord(axis.cross()) >= size::SCROLLBAR_WIDTH;
        let view = self.view();
        let overflows = |child: Size| room && child.get_coord(axis) > view + 0.5;
        let bar = |overflow: bool| if overflow { size::SCROLLBAR_WIDTH } else { 0.0 };
        let assumed = self.overflow && room;
        let mut child = self.layout_child(ctx, size, bar(assumed));
        self.overflow = overflows(child);
        if self.overflow != assumed {
            child = self.layout_child(ctx, size, bar(self.overflow));
            self.overflow = overflows(child) || self.overflow && !assumed;
        }
        ctx.run_layout(&mut self.child, child);
        self.content = child.get_coord(axis);
        self.clamp();
        ctx.set_clip_path(size.to_rect());
        ctx.place_child(&mut self.child, Point::ZERO);
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        let scale = ctx.scale_factor();
        let offset = (self.offset * scale).round() / scale;
        let shift = self.shift(offset);
        ctx.set_animated_child_scroll_translation(&mut self.child, shift);
        if self.axis == Axis::Vertical {
            aq_trace::counter("прокрутка: смещение", self.offset);
            let id = ctx.widget_id().to_raw();
            crate::render::scroll::publish(id, ctx.window_transform().transform_rect_bbox(self.viewport.to_rect()), offset);
            self.published = Some(id);
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn post_paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if !self.overflow {
            return;
        }
        crate::render::sprite::paint(&self.thumb, self.bar.color());
        painter.fill(self.bar.shape(&self.span()), &self.thumb).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::ScrollView
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_clips_children();
        match self.axis {
            Axis::Vertical => {
                node.set_scroll_y(self.offset);
                node.set_scroll_y_min(0.0);
                node.set_scroll_y_max(self.max_offset());
            }
            Axis::Horizontal => {
                node.set_scroll_x(self.offset);
                node.set_scroll_x_min(0.0);
                node.set_scroll_x_max(self.max_offset());
            }
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
