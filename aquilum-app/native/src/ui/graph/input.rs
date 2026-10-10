use super::*;

const PINCH_RESPONSE: f64 = 0.01;
const WHEEL_RESPONSE: f64 = 0.0018;
const NOTCH_PIXELS: f64 = 100.0;

impl GraphView {
    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        if self.graph.is_none() {
            return;
        }
        let in_panel = |p: Point| self.settings.is_some() && self.panel.contains(p);
        match event {
            PointerEvent::Down(down) if down.button.is_none_or(|b| b == PointerButton::Primary) => {
                let p = ctx.local_position(down.state.position);
                if in_panel(p) {
                    return;
                }
                if down.state.count == 2 {
                    self.glide();
                    ctx.request_anim_frame();
                    return;
                }
                self.drag = Some(Drag { last: p, travelled: 0.0 });
                ctx.capture_pointer();
            }
            PointerEvent::Move(update) => {
                let p = ctx.local_position(update.current.position);
                if let Some(drag) = &mut self.drag {
                    let delta = p - drag.last;
                    drag.last = p;
                    drag.travelled += delta.x.abs() + delta.y.abs();
                    if drag.travelled > DRAG_THRESHOLD {
                        self.camera.pan_by(delta.x, delta.y);
                        self.hover(None);
                        self.moved();
                        ctx.request_anim_frame();
                    }
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
                if drag.travelled <= DRAG_THRESHOLD
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
                    ScrollDelta::LineDelta(_, y) => self.camera.zoom_by((f64::from(*y) * NOTCH_PIXELS * WHEEL_RESPONSE).exp(), cx, cy),
                    ScrollDelta::PixelDelta(d) if state.modifiers.ctrl() || state.modifiers.meta() => self.camera.zoom_by((d.y / scale * PINCH_RESPONSE).exp(), cx, cy),
                    ScrollDelta::PixelDelta(d) => self.camera.pan_by(d.x / scale, d.y / scale),
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
