use std::time::Instant;

use super::{Curve, Fling};
use crate::ui::keymap::gesture::FLING_FRICTION;

const INFLEXION: f64 = 0.35;
const START_TENSION: f64 = 0.5;
const END_TENSION: f64 = 1.0;
const P1: f64 = START_TENSION * INFLEXION;
const P2: f64 = 1.0 - END_TENSION * (1.0 - INFLEXION);
const PHYSICAL_PER_DP: f64 = 9.80665 * 39.37 * 160.0 * 0.84;

pub(super) struct Spline {
    distance: f64,
    pub(super) duration: f64,
}

impl Spline {
    pub(super) fn distance(&self, ms: f64) -> f64 {
        self.distance * position((ms / self.duration).clamp(0.0, 1.0))
    }
}

fn cubic(x: f64, a: f64, b: f64) -> f64 {
    3.0 * x * (1.0 - x) * ((1.0 - x) * a + x * b) + x * x * x
}

fn position(alpha: f64) -> f64 {
    let (low, high) = (0..40).fold((0.0, 1.0), |(low, high), _| {
        let x = (low + high) / 2.0;
        if cubic(x, P1, P2) > alpha { (low, x) } else { (x, high) }
    });
    cubic((low + high) / 2.0, START_TENSION, 1.0)
}

impl Fling {
    pub fn spline(start: Instant, velocity: (f64, f64), dp: f64) -> Self {
        let rate = 0.78f64.ln() / 0.9f64.ln();
        let base = FLING_FRICTION * PHYSICAL_PER_DP * dp;
        let deceleration = (INFLEXION * velocity.0.hypot(velocity.1) / base).ln();
        let duration = 1000.0 * (deceleration / (rate - 1.0)).exp();
        let distance = base * (rate / (rate - 1.0) * deceleration).exp();
        Fling::new(start, velocity, Curve::Spline(Spline { distance, duration }))
    }
}
