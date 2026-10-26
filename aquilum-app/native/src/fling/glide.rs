#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use super::trail::slope;
use super::{Curve, Fling, Trail};

const BEND_MS: f64 = 150.0;
const POWER: f64 = 1.32;
const FRICTION_PER_MS: f64 = 0.165;

pub(super) struct Glide {
    speed: f64,
    friction: f64,
    pub(super) end: f64,
}

impl Glide {
    pub(super) fn distance(&self, ms: f64) -> f64 {
        let ms = ms.clamp(0.0, self.end);
        self.speed * BEND_MS / (POWER - 1.0) * (1.0 - (1.0 + ms / BEND_MS).powf(1.0 - POWER)) - self.friction * ms
    }
}

impl Trail {
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn release(&mut self, scale: f64) -> Option<Fling> {
        let mut samples = std::mem::take(&mut self.samples);
        let (last, x, y) = samples.pop_back()?;
        let &(from, px, py) = samples.back()?;
        let ms = |t: Instant| -from.saturating_duration_since(t).as_secs_f64() * 1000.0;
        let (vx, vy) = slope(samples.iter().map(|s| (ms(s.0), s.1, s.2)), false)?;
        let measured = vx.hypot(vy);
        let friction = FRICTION_PER_MS * scale;
        if measured <= friction {
            return None;
        }
        let lift = (((x - px) * vx + (y - py) * vy) / measured / measured).clamp(0.0, last.saturating_duration_since(from).as_secs_f64() * 1000.0);
        let start = from + Duration::from_secs_f64(lift / 1000.0);
        let speed = measured + friction;
        let end = BEND_MS * ((speed / friction).powf(1.0 / POWER) - 1.0);
        Some(Fling::new(start, (vx, vy), Curve::Glide(Glide { speed, friction, end })))
    }
}
