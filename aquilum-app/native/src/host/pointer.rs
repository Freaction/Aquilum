use super::*;

const DOUBLE_CLICK: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_SLOP: f64 = 4.0;

impl Host {
    pub(super) fn time_ns(&self) -> u64 {
        self.started.elapsed().as_nanos() as u64
    }

    pub(super) fn click_count(&mut self, button: Option<PointerButton>, position: winit::dpi::PhysicalPosition<f64>) -> u8 {
        let now = Instant::now();
        let slop = DOUBLE_CLICK_SLOP * self.window().scale_factor();
        let count = match self.click {
            Some((last, at, from, count)) if button == Some(last) && now - at <= DOUBLE_CLICK && (from.x - position.x).abs() <= slop && (from.y - position.y).abs() <= slop => count.saturating_add(1),
            _ => 1,
        };
        self.click = button.map(|b| (b, now, position, count));
        count
    }

    pub(super) fn pointer_state(&mut self) -> PointerState {
        self.pointer.time = self.time_ns();
        self.pointer.modifiers = self.modifiers;
        self.pointer.scale_factor = self.scale();
        self.pointer.clone()
    }

    pub(super) fn pinch(&mut self, delta: f64) {
        if !delta.is_finite() || delta == 0.0 {
            return;
        }
        let state = self.pointer_state();
        let gesture = masonry::core::PointerGestureEvent { pointer: PRIMARY_MOUSE, gesture: masonry::core::PointerGesture::Pinch(delta as f32), state };
        self.pointer_event(PointerEvent::Gesture(gesture));
    }

    pub(super) fn pointer_event(&mut self, event: PointerEvent) {
        if let Some(root) = &mut self.root {
            root.handle_pointer_event(event);
        }
    }

    pub(super) fn pointer_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        self.update_resize_edge(position);
        self.pointer.position = position;
        self.moved = true;
        self.vsync.request();
    }

    pub(super) fn flush_move(&mut self) {
        if std::mem::take(&mut self.moved) {
            let current = self.pointer_state();
            self.pointer_event(PointerEvent::Move(PointerUpdate {
                pointer: PRIMARY_MOUSE,
                current,
                coalesced: vec![],
                predicted: vec![],
            }));
        }
    }

    pub(super) fn pointer_button(&mut self, state: ElementState, position: winit::dpi::PhysicalPosition<f64>, button: winit::event::ButtonSource) {
        self.pointer.position = position;
        let button = button.mouse_button().and_then(|b| match b {
            MouseButton::Left => Some(PointerButton::Primary),
            MouseButton::Right => Some(PointerButton::Secondary),
            MouseButton::Middle => Some(PointerButton::Auxiliary),
            MouseButton::Back => Some(PointerButton::X1),
            MouseButton::Forward => Some(PointerButton::X2),
            _ => None,
        });
        let pressed = state == ElementState::Pressed;
        if let Some(b) = button {
            if pressed {
                self.pointer.buttons.insert(b);
            } else {
                self.pointer.buttons.remove(b);
            }
        }
        if pressed {
            self.pointer.count = self.click_count(button, position);
        }
        let state = self.pointer_state();
        let event = PointerButtonEvent { pointer: PRIMARY_MOUSE, button, state };
        self.pointer_event(if pressed { PointerEvent::Down(event) } else { PointerEvent::Up(event) });
        let delta = match button {
            Some(PointerButton::X1) => -1,
            Some(PointerButton::X2) => 1,
            _ => 0,
        };
        if !pressed
            && delta != 0
            && let Some(root) = &mut self.root
            && self.app.navigate(root, delta).title
        {
            self.show_title();
        }
    }

    pub(super) fn wheel_zoom(&mut self, up: bool) {
        let rescale = self.app.zoom(up);
        self.apply(crate::app::Outcome { rescale, ..Default::default() });
    }

    pub(super) fn wheel_delta(&mut self, delta: MouseScrollDelta) -> Option<ScrollDelta> {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                aq_trace::counter("колесо: строки", f64::from(y));
                Some(ScrollDelta::LineDelta(x, y))
            }
            MouseScrollDelta::PixelDelta(p) => {
                if self.touchpad.as_ref().is_some_and(crate::touchpad::Touchpad::active) {
                    return None;
                }
                aq_trace::counter("колесо: пиксели", p.y);
                Some(ScrollDelta::PixelDelta(p))
            }
            _ => None,
        }
    }
}
