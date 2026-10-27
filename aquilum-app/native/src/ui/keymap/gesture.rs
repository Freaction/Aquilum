use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    LongPress,
    MouseBack,
    MouseForward,
}

pub const TOUCH_SLOP: f64 = 8.0;
pub const LONG_PRESS: Duration = Duration::from_millis(400);
pub const DOUBLE_TAP: Duration = Duration::from_millis(300);
pub const DOUBLE_TAP_SLOP: f64 = 100.0;
pub const DOUBLE_CLICK: Duration = Duration::from_millis(500);
pub const DOUBLE_CLICK_SLOP: f64 = 4.0;
pub const DRAG_SLOP: f64 = 4.0;
pub const GRIP: f64 = 6.0;
pub const TOUCH_GRIP: f64 = 14.0;
pub const TOUCH_REACH: f64 = 8.0;
pub const EDGE_SCROLL_RATE: f64 = 10.0;
pub const EDGE_SCROLL_REACH: f64 = 300.0;
pub const TOUCH_EDGE: f64 = 40.0;
pub const WHEEL_LINE: f64 = 48.0;
pub const WHEEL_PAGE_TURN: f64 = 120.0;
pub const PAGE_GESTURE_GAP: Duration = Duration::from_millis(250);
pub const PAGE_ZONE: f64 = 0.08;
pub const PAGE_ZONE_MIN: f64 = 40.0;
pub const PAGE_ZONE_MAX: f64 = 72.0;
pub const WHEEL_ZOOM: f64 = 0.18;
pub const PINCH_ZOOM: f64 = 0.01;
pub const FLING_MIN_VELOCITY: f64 = 50.0;
pub const FLING_MAX_VELOCITY: f64 = 8000.0;
pub const FLING_FRICTION: f64 = 0.015;
pub const SCALING_SPAN_SLOP: f64 = 2.0 * TOUCH_SLOP;
pub const MIN_SCALING_SPAN: f64 = 170.0;
