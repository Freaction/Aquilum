use super::*;
use crate::ui::keymap::gesture::{PINCH_ZOOM, WHEEL_ZOOM};
use crate::ui::keymap::pointer;

impl GraphView {
    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        let over = match event {
            PointerEvent::Move(update) => self.action_rect.is_some_and(|r| r.contains(ctx.local_position(update.current.position))),
            PointerEvent::Leave(_) => false,
            _ => self.over_action,
        };
        if over != self.over_action {
            self.over_action = over;
            ctx.request_paint_only();
        }
        if self.graph.is_none() {
            if let PointerEvent::Down(down) = event
                && pointer::press(down.button)
            {
                let p = ctx.local_position(down.state.position);
                if self.action_rect.is_some_and(|r| r.contains(p)) && matches!(self.status, Status::Failed(_)) {
                    ctx.submit_action::<GraphAction>(GraphAction::Retry);
                }
            }
            return;
        }
        let in_panel = |p: Point| self.settings.is_some() && self.panel.contains(p);
        match event {
            PointerEvent::Down(down) if pointer::press(down.button) => {
                let p = ctx.local_position(down.state.position);
                if in_panel(p) {
                    return;
                }
                if self.action_rect.is_some_and(|r| r.contains(p)) && self.visible == 0 {
                    ctx.submit_action::<GraphAction>(GraphAction::ShowAll);
                    return;
                }
                if down.state.count == 2 {
                    self.glide();
                    ctx.request_anim_frame();
                    return;
                }
                self.drag = Some(Drag { last: p, travelled: 0.0, slop: pointer::drag_slop(down.pointer.pointer_type) });
                ctx.capture_pointer();
            }
            PointerEvent::Move(update) => {
                let p = ctx.local_position(update.current.position);
                if let Some(drag) = &mut self.drag {
                    let delta = p - drag.last;
                    drag.last = p;
                    drag.travelled += delta.x.abs() + delta.y.abs();
                    if drag.panning() {
                        self.camera.pan_by(delta.x, delta.y);
                        self.hover(None);
                        self.moved();
                        ctx.request_anim_frame();
                    }
                    return;
                }
                if update.pointer.pointer_type == masonry::core::PointerType::Touch {
                    return;
                }
                if in_panel(p) {
                    self.pointer = None;
                    if self.hover(None) {
                        ctx.request_anim_frame();
                    }
                    return;
                }
                self.pointer = Some(p);
                if self.hover(self.node_at(p)) {
                    ctx.request_anim_frame();
                }
            }
            PointerEvent::Up(up) => {
                let Some(drag) = self.drag.take() else { return };
                if !drag.panning()
                    && let Some(node) = self.node_at(ctx.local_position(up.state.position))
                    && let Some(path) = self.graph.as_ref().map(|g| g.paths[node].clone())
                {
                    ctx.submit_action::<GraphAction>(GraphAction::Open(path));
                }
            }
            PointerEvent::Leave(_) => {
                self.pointer = None;
                if self.drag.is_none() && self.hover(None) {
                    ctx.request_anim_frame();
                }
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, state, .. }) => {
                let p = ctx.local_position(state.position);
                if in_panel(p) {
                    return;
                }
                let (cx, cy) = (p.x - self.size.width / 2.0, p.y - self.size.height / 2.0);
                let scale = ctx.scale_factor().max(0.1);
                match delta {
                    ScrollDelta::LineDelta(_, y) => self.camera.zoom_by((f64::from(*y) * WHEEL_ZOOM).exp(), cx, cy),
                    ScrollDelta::PixelDelta(_) if pointer::zoom(state.modifiers) => self.camera.zoom_by((crate::ui::scroll::pixels(delta, scale, 0.0).y * PINCH_ZOOM).exp(), cx, cy),
                    ScrollDelta::PixelDelta(_) => {
                        let d = crate::ui::scroll::pixels(delta, scale, 0.0);
                        self.camera.pan_by(d.x, d.y);
                    }
                    _ => return,
                }
                self.moved();
                ctx.request_anim_frame();
                ctx.set_handled();
            }
            PointerEvent::Gesture(gesture) => {
                if let masonry::core::PointerGesture::Pinch(delta) = gesture.gesture {
                    let p = ctx.local_position(gesture.state.position);
                    if in_panel(p) {
                        return;
                    }
                    self.camera.zoom_by((1.0 + f64::from(delta)).max(0.01), p.x - self.size.width / 2.0, p.y - self.size.height / 2.0);
                    self.moved();
                    ctx.request_anim_frame();
                    ctx.set_handled();
                }
            }
            _ => {}
        }
    }
}
