use masonry::core::{EventCtx, PointerButton, PointerEvent, PointerScrollEvent, ScrollDelta, Widget};
use masonry::kurbo::Axis;

use super::area::ScrollArea;
use super::{pixels, sideways};
use crate::ui::keymap::pointer;

impl<W: Widget> ScrollArea<W> {

    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Scroll(PointerScrollEvent { delta, state, .. }) => {
                let before = (self.offset, self.glide.map(|g| g.to));
                let d = pixels(delta, ctx.scale_factor(), self.view());
                let d = if self.axis == Axis::Vertical { d.y } else { sideways(d, pointer::sideways(state.modifiers)) };
                if matches!(delta, ScrollDelta::PixelDelta(_)) {
                    self.jump(self.offset - d);
                } else if self.glide_by(-d) {
                    ctx.request_anim_frame();
                }
                if (self.offset, self.glide.map(|g| g.to)) != before {
                    ctx.request_compose();
                    ctx.request_post_paint();
                    ctx.set_handled();
                }
            }
            PointerEvent::Down(e) if pointer::press(e.button) && self.overflow => {
                let at = ctx.local_position(e.state.position);
                let span = self.span();
                let pressed = self.bar.press(at, &span).filter(|_| self.bar.dragging() || pointer::pages_track(e.pointer.pointer_type));
                if let Some(next) = pressed {
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
                let (along, zone) = (at.get_coord(self.axis), pointer::edge(update.pointer.pointer_type));
                let held = ctx.has_active() && !ctx.is_active() && update.current.buttons.contains(PointerButton::Primary);
                self.edge = if held { (along - zone).min(0.0) + (along + zone - self.view()).max(0.0) } else { 0.0 };
                if self.edge != 0.0 {
                    ctx.request_anim_frame();
                }
                if let Some(next) = self.bar.drag(at, &self.span()) {
                    self.offset = next;
                    ctx.request_compose();
                    ctx.request_post_paint();
                }
                let span = self.span();
                if self.bar.hover(self.overflow.then_some(at), &span) {
                    ctx.request_post_paint();
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                self.edge = 0.0;
                if self.bar.release() {
                    ctx.request_post_paint();
                }
            }
            PointerEvent::Leave(_) if !self.bar.dragging() => {
                let span = self.span();
                if self.bar.hover(None, &span) {
                    ctx.request_post_paint();
                }
            }
            _ => {}
        }
    }
}
