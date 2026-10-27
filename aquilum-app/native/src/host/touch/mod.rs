mod detector;
mod scale;
#[cfg(test)]
mod tests;

use masonry::core::pointer::PointerButtons;
use masonry::kurbo::Point;
use winit::dpi::PhysicalPosition;
use winit::event::FingerId;

use super::*;
use crate::ui::keymap::{Gesture, pointer};

pub(super) use detector::Touch;

const TOUCH: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Touch,
};

fn point(position: PhysicalPosition<f64>) -> Point {
    Point::new(position.x, position.y)
}

impl Host {
    pub(super) fn touch_button(&mut self, finger: FingerId, state: ElementState, position: PhysicalPosition<f64>) {
        let now = Instant::now();
        self.touch_due(now);
        let (at, held) = (point(position), self.touch.held());
        if state == ElementState::Pressed {
            let dp = self.window().scale_factor();
            self.touch.down(finger.into_raw(), at, now, dp);
            if held.is_some() {
                self.pointer_event(PointerEvent::Cancel(TOUCH));
            } else if self.touch.pressed() && self.grabs(at) {
                self.touch.hold(false);
                self.touch_button_event(at, PointerButton::Primary, 1, true);
            }
        } else if let Some((at, count)) = self.touch.up(finger.into_raw(), at, now) {
            self.touch_click(at, PointerButton::Primary, count);
        } else if let Some(menu) = held.filter(|_| self.touch.held().is_none()) {
            self.touch_button_event(at, PointerButton::Primary, 1, false);
            self.pointer_event(PointerEvent::Leave(TOUCH));
            if menu {
                self.touch_click(at, PointerButton::Secondary, 1);
            }
        }
    }

    pub(super) fn touch_moved(&mut self, finger: FingerId, position: PhysicalPosition<f64>) {
        let now = Instant::now();
        self.touch_due(now);
        self.touch.moved(finger.into_raw(), point(position), now);
        if self.touch.held().is_some() {
            self.touch_move(point(position), PointerButton::Primary.into());
        }
    }

    pub(super) fn touch_left(&mut self, finger: FingerId) {
        if self.touch.held().is_some() {
            self.pointer_event(PointerEvent::Cancel(TOUCH));
        }
        self.touch.cancel(finger.into_raw());
    }

    pub(super) fn touch_due(&mut self, now: Instant) {
        let Some(at) = self.touch.long_press(now) else { return };
        if self.root.as_ref().is_some_and(|root| pointer::selects(root.cursor_icon())) {
            self.touch.hold(true);
            self.touch_button_event(at, PointerButton::Primary, 2, true);
        } else {
            self.run_gesture(Gesture::LongPress, at);
        }
    }

    pub(super) fn touch_frame(&mut self, event_loop: &dyn ActiveEventLoop, vblank: Instant) {
        let frame = self.touch.frame(vblank);
        if let Some((at, delta)) = frame.scroll {
            let state = self.touch_state(at);
            let delta = ScrollDelta::PixelDelta(PhysicalPosition::new(delta.x, delta.y));
            self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: TOUCH, delta, state }));
        }
        if let Some((at, delta)) = frame.pinch {
            let state = self.touch_state(at);
            self.pinch_at(TOUCH, state, delta);
        }
        if frame.scroll.is_some() || frame.pinch.is_some() {
            self.pointer_event(PointerEvent::Leave(TOUCH));
            self.handle_signals(event_loop);
        }
    }

    pub(super) fn touch_click(&mut self, at: Point, button: PointerButton, count: u8) {
        self.touch_move(at, PointerButtons::new());
        #[cfg(target_os = "android")]
        let text = button == PointerButton::Primary && self.root.as_ref().is_some_and(|root| pointer::selects(root.cursor_icon()));
        self.touch_button_event(at, button, count, true);
        self.touch_button_event(at, button, count, false);
        self.pointer_event(PointerEvent::Leave(TOUCH));
        #[cfg(target_os = "android")]
        if text {
            self.ime_tapped();
        }
    }

    fn grabs(&mut self, at: Point) -> bool {
        self.touch_move(at, PointerButtons::new());
        self.root.as_ref().is_some_and(|root| pointer::drags(root.cursor_icon()))
    }

    fn touch_move(&mut self, at: Point, buttons: PointerButtons) {
        let current = PointerState { buttons, ..self.touch_state(at) };
        self.pointer_event(PointerEvent::Move(PointerUpdate { pointer: TOUCH, current, coalesced: vec![], predicted: vec![] }));
    }

    fn touch_button_event(&mut self, at: Point, button: PointerButton, count: u8, down: bool) {
        let state = PointerState { count, ..self.touch_state(at) };
        let event = |state| PointerButtonEvent { pointer: TOUCH, button: Some(button), state };
        self.pointer_event(if down {
            PointerEvent::Down(event(PointerState { buttons: button.into(), ..state }))
        } else {
            PointerEvent::Up(event(state))
        });
    }

    fn touch_state(&mut self, at: Point) -> PointerState {
        PointerState { position: PhysicalPosition::new(at.x, at.y), buttons: PointerButtons::new(), ..self.pointer_state() }
    }
}
