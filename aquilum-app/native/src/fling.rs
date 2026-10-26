#[cfg(target_os = "windows")]
mod glide;
mod spline;
#[cfg(test)]
mod tests;
mod trail;

use std::time::Instant;

pub use trail::Trail;

enum Curve {
    #[cfg(target_os = "windows")]
    Glide(glide::Glide),
    Spline(spline::Spline),
}

pub struct Fling {
    start: Instant,
    direction: (f64, f64),
    curve: Curve,
    covered: f64,
}

impl Fling {
    fn new(start: Instant, (vx, vy): (f64, f64), curve: Curve) -> Self {
        let speed = vx.hypot(vy);
        Fling { start, direction: (vx / speed, vy / speed), curve, covered: 0.0 }
    }

    pub fn advance(&mut self, at: Instant) -> Option<(f64, f64)> {
        let ms = at.saturating_duration_since(self.start).as_secs_f64() * 1000.0;
        let (end, distance) = match &self.curve {
            #[cfg(target_os = "windows")]
            Curve::Glide(glide) => (glide.end, glide.distance(ms)),
            Curve::Spline(spline) => (spline.duration, spline.distance(ms)),
        };
        let step = distance - self.covered;
        self.covered = distance;
        (ms < end || step > 0.0).then_some((self.direction.0 * step, self.direction.1 * step))
    }
}
