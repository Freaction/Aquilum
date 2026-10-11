use std::time::{Duration, Instant};

use crate::ui::widgets::bezier;

const MIN_SECONDS: f64 = 0.05;
const MAX_SECONDS: f64 = 0.2;
const INTERVAL_RATIO: f64 = 2.0;
const VELOCITY_WEIGHT: f64 = 0.25;
const STOP_WEIGHT: f64 = 0.4;

#[derive(Clone, Copy)]
pub struct Glide {
    from: f64,
    pub to: f64,
    duration: f64,
    elapsed: f64,
    curve: (f64, f64, f64, f64),
}

impl Glide {
    fn new(from: f64, to: f64, velocity: f64, duration: f64) -> Self {
        let span = to - from;
        let slope = if span == 0.0 || velocity * span <= 0.0 { 0.0 } else { velocity * duration / span };
        let norm = (1.0 + slope * slope).sqrt();
        let curve = (VELOCITY_WEIGHT / norm, VELOCITY_WEIGHT * slope / norm, 1.0 - STOP_WEIGHT, 1.0);
        Glide { from, to, duration, elapsed: 0.0, curve }
    }

    fn at(&self, elapsed: f64) -> f64 {
        let t = (elapsed / self.duration).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * bezier(self.curve, t)
    }

    pub fn shift(&mut self, dy: f64) {
        self.from += dy;
        self.to += dy;
    }

    pub fn position(&self) -> f64 {
        self.at(self.elapsed)
    }

    fn velocity(&self) -> f64 {
        let h = 0.001;
        (self.at(self.elapsed + h) - self.at(self.elapsed - h)) / (2.0 * h)
    }

    pub fn advance(&mut self, seconds: f64) -> bool {
        self.elapsed += seconds;
        self.elapsed < self.duration
    }
}

#[derive(Default)]
pub struct Wheel {
    history: Vec<Instant>,
}

impl Wheel {
    pub fn retarget(&mut self, glide: Option<Glide>, offset: f64, delta: f64, max: f64) -> Option<Glide> {
        let now = Instant::now();
        if self.history.last().is_none_or(|last| now - *last > Duration::from_secs_f64(MAX_SECONDS * 2.0)) {
            self.history = vec![now - Duration::from_secs_f64(MAX_SECONDS); 3];
        }
        let interval = (now - self.history[0]).as_secs_f64() / 3.0;
        self.history.remove(0);
        self.history.push(now);
        let duration = (interval * INTERVAL_RATIO).clamp(MIN_SECONDS, MAX_SECONDS);
        let (base, velocity) = match glide {
            Some(g) if (g.to - offset) * delta > 0.0 => (g.to, g.velocity()),
            Some(g) => (offset, g.velocity()),
            None => (offset, 0.0),
        };
        let to = (base + delta).clamp(0.0, max);
        ((to - offset).abs() >= 0.5).then(|| Glide::new(offset, to, velocity, duration))
    }
}
