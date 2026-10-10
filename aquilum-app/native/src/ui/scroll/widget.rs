//! Реализация трейта `Widget` для `ScrollArea`.

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, ComposeCtx, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerButton, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef, RegisterCtx,
    ScrollDelta, Update, UpdateCtx, Widget,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size, Vec2};
use masonry::layout::{LayoutSize, LenReq, Length};

use super::area::ScrollArea;
use super::bar;
use super::{GLIDE_SECONDS, LINE, Scrolled};
use crate::ui::tokens::size;

impl<W: Widget> ScrollArea<W> {
    fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        let local = |ctx: &EventCtx<'_>, position| ctx.local_position(position);
        match event {
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                let before = (self.offset, self.glide.map(|g| g.to));
                match *delta {
                    ScrollDelta::LineDelta(_, y) => {
                        if self.glide_by(-f64::from(y) * LINE) {
                            ctx.request_anim_frame();
                        }
                    }
                    ScrollDelta::PixelDelta(p) => self.jump(self.offset - p.y / ctx.scale_factor()),
                    ScrollDelta::PageDelta(_, y) => {
                        if self.glide_by(-f64::from(y) * self.viewport.height) {
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
                let at = local(ctx, e.state.position);
                let track = bar::track(self.viewport);
                if track.contains(at) {
                    let thumb = bar::thumb(self.viewport, self.content_height, self.offset, self.max_offset());
                    if thumb.contains(at) {
                        self.glide = None;
                        self.dragging = Some((at.y, self.offset));
                        ctx.capture_pointer();
                    } else {
                        // Щелчок по дорожке — страница к указателю.
                        let page = self.viewport.height * bar::PAGE;
                        self.jump(self.offset + if at.y < thumb.y0 { -page } else { page });
                        ctx.request_compose();
                    }
                    ctx.request_post_paint();
                    ctx.set_handled();
                }
            }
            PointerEvent::Move(update) => {
                let at = local(ctx, update.current.position);
                if let Some((start, offset)) = self.dragging {
                    let track = bar::track(self.viewport);
                    let thumb = bar::thumb(self.viewport, self.content_height, self.offset, self.max_offset());
                    let travel = (track.height() - bar::THUMB_INSET * 2.0 - thumb.height()).max(1.0);
                    self.offset = offset + (at.y - start) / travel * self.max_offset();
                    self.clamp();
                    ctx.request_compose();
                    ctx.request_post_paint();
                }
                let thumb = bar::thumb(self.viewport, self.content_height, self.offset, self.max_offset());
                let hovered = self.overflow && thumb.contains(at);
                if hovered != self.thumb_hovered {
                    self.thumb_hovered = hovered;
                    ctx.request_post_paint();
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) if self.dragging.is_some() => {
                self.dragging = None;
                ctx.request_post_paint();
            }
            PointerEvent::Leave(_) if self.thumb_hovered && self.dragging.is_none() => {
                self.thumb_hovered = false;
                ctx.request_post_paint();
            }
            _ => {}
        }
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
        glide.elapsed += interval as f64 / 1e9;
        self.offset = glide.at(GLIDE_SECONDS).0.clamp(0.0, self.max_offset());
        self.glide = (glide.elapsed < GLIDE_SECONDS).then_some(glide);
        if self.glide.is_some() {
            ctx.request_anim_frame();
        }
        ctx.request_compose();
        ctx.request_post_paint();
        ctx.submit_action::<Scrolled>(Scrolled(self.offset));
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::RequestPanToChild(target) = event {
            self.glide = None;
            let (top, bottom) = (target.y0, target.y1);
            if top < self.offset {
                self.offset = top;
            } else if bottom > self.offset + self.viewport.height {
                self.offset = bottom - self.viewport.height;
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
        match len_req {
            LenReq::MinContent => Length::ZERO,
            LenReq::FitContent(space) => space,
            LenReq::MaxContent => {
                let context = LayoutSize::maybe(axis.cross(), cross);
                let cross_space = cross.filter(|_| axis == Axis::Vertical);
                ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross_space)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        self.viewport = size;
        let bar = size.width >= size::SCROLLBAR_WIDTH;
        let width = |overflow: bool| if overflow { (size.width - size::SCROLLBAR_WIDTH).max(0.0) } else { size.width };
        let overflows = |height: f64| bar && height > size.height + 0.5;
        let assumed = self.overflow && bar;
        let mut child = self.layout_child(ctx, width(assumed), size.height);
        self.overflow = overflows(child.height);
        if self.overflow != assumed {
            child = self.layout_child(ctx, width(self.overflow), size.height);
            self.overflow = overflows(child.height) || self.overflow && !assumed;
        }
        ctx.run_layout(&mut self.child, child);
        self.content_height = child.height;
        self.clamp();
        ctx.set_clip_path(size.to_rect());
        ctx.place_child(&mut self.child, Point::ZERO);
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        let scale = ctx.scale_factor();
        aq_trace::counter("прокрутка: смещение", self.offset);
        ctx.set_child_scroll_translation(&mut self.child, Vec2::new(0.0, -(self.offset * scale).round() / scale));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn post_paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if !self.overflow {
            return;
        }
        let track = bar::track(self.viewport);
        let thumb = bar::thumb(self.viewport, self.content_height, self.offset, self.max_offset());
        let active = self.thumb_hovered || self.dragging.is_some();
        bar::paint_scrollbar(painter, track, thumb, active);
    }

    fn accessibility_role(&self) -> Role {
        Role::ScrollView
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_clips_children();
        node.set_scroll_y(self.offset);
        node.set_scroll_y_min(0.0);
        node.set_scroll_y_max(self.max_offset());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
