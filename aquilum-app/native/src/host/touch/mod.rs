mod detector;
mod scale;
#[cfg(test)]
mod tests;

use masonry::core::pointer::PointerButtons;
use masonry::kurbo::Point;
use winit::dpi::PhysicalPosition;
use winit::event::FingerId;

use super::*;
use crate::ui::keymap::Gesture;

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
        if state == ElementState::Pressed {
            let dp = self.window().scale_factor();
            self.touch.down(finger.into_raw(), point(position), now, dp);
        } else if let Some((at, count)) = self.touch.up(finger.into_raw(), point(position), now) {
            self.touch_click(at, PointerButton::Primary, count);
        }
    }

    pub(super) fn touch_moved(&mut self, finger: FingerId, position: PhysicalPosition<f64>) {
        let now = Instant::now();
        self.touch_due(now);
        self.touch.moved(finger.into_raw(), point(position), now);
    }

    pub(super) fn touch_left(&mut self, finger: FingerId) {
        self.touch.cancel(finger.into_raw());
    }

    pub(super) fn touch_due(&mut self, now: Instant) {
        if let Some(at) = self.touch.long_press(now) {
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
        let state = PointerState { count, ..self.touch_state(at) };
        self.pointer_event(PointerEvent::Move(PointerUpdate { pointer: TOUCH, current: state.clone(), coalesced: vec![], predicted: vec![] }));
        let pressed = PointerState { buttons: button.into(), ..state.clone() };
        self.pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: TOUCH, button: Some(button), state: pressed }));
        self.pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: TOUCH, button: Some(button), state }));
        self.pointer_event(PointerEvent::Leave(TOUCH));
    }

    fn touch_state(&mut self, at: Point) -> PointerState {
        PointerState { position: PhysicalPosition::new(at.x, at.y), buttons: PointerButtons::new(), ..self.pointer_state() }
    }
}
