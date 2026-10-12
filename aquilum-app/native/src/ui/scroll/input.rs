use masonry::core::{EventCtx, PointerButton, PointerEvent, PointerScrollEvent, ScrollDelta, Widget};
use masonry::kurbo::Axis;

use super::LINE;
use super::area::ScrollArea;

impl<W: Widget> ScrollArea<W> {
    fn wheel_delta(&self, x: f64, y: f64, shift: bool) -> f64 {
        match self.axis {
            Axis::Vertical => y,
            Axis::Horizontal if shift && x == 0.0 => y,
            Axis::Horizontal if x.abs() > y.abs() => x,
            Axis::Horizontal => 0.0,
        }
    }

    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
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
                if self.bar.hover(self.overflow.then_some(at), &span) {
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
}
