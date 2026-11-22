use std::collections::VecDeque;
use std::time::{Duration, Instant};

const TOUCHPAD_HORIZON: Duration = Duration::from_millis(40);
/// Recent window for fling speed (Impulse-style): inherit the finger's speed at lift.
const TOUCH_HORIZON: Duration = Duration::from_millis(50);
const STOPPED: Duration = Duration::from_millis(40);
/// Touch moves from one OS burst share nearly identical Instant::now() stamps.
/// Collapse sub‑millisecond stamps so velocity stays in px/s, not px/µs, while
/// keeping real ~120 Hz samples from distinct frames.
const MIN_SAMPLE_GAP: Duration = Duration::from_millis(1);
const HISTORY: usize = 20;

pub struct Trail {
    pub(super) samples: VecDeque<(Instant, f64, f64)>,
    horizon: Duration,
}

impl Default for Trail {
    fn default() -> Self {
        Trail { samples: VecDeque::new(), horizon: TOUCHPAD_HORIZON }
    }
}

impl Trail {
    pub fn touch() -> Self {
        Trail { horizon: TOUCH_HORIZON, ..Trail::default() }
    }

    #[allow(dead_code)] // used by the Windows touchpad path
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, at: Instant, x: f64, y: f64) {
        if let Some(&(t, px, py)) = self.samples.back() {
            let dt = at.saturating_duration_since(t);
            // Only treat a gap as "finger stopped" when the position did not move.
            // Sparse sampling must not wipe a continuous swipe's history.
            if dt > STOPPED && (x - px).hypot(y - py) < 1.0 {
                self.samples.clear();
            } else if dt < MIN_SAMPLE_GAP {
                // Same OS poll / Instant burst: keep the latest point at this slot.
                self.samples.pop_back();
            }
        }
        self.samples.push_back((at, x, y));
        while self.samples.len() > HISTORY || self.samples.front().is_some_and(|&(t, _, _)| at.saturating_duration_since(t) > self.horizon) {
            self.samples.pop_front();
        }
    }

    pub fn velocity(&self, at: Instant) -> Option<(f64, f64)> {
        let &(newest, _, _) = self.samples.back()?;
        if at.saturating_duration_since(newest) > STOPPED {
            return None;
        }
        let age = |t: Instant| -newest.saturating_duration_since(t).as_secs_f64();
        slope(self.samples.iter().map(|&(t, x, y)| (age(t), x, y)))
    }
}

// A linear fit cannot invent a reverse velocity for monotonic movement.
pub(super) fn slope(points: impl Iterator<Item = (f64, f64, f64)>) -> Option<(f64, f64)> {
    let mut t = [0.0; 3];
    let mut sums = [[0.0; 2]; 2];
    for (ti, x, y) in points {
        t[0] += 1.0;
        t[1] += ti;
        t[2] += ti * ti;
        for (axis, v) in sums.iter_mut().zip([x, y]) {
            axis[0] += v;
            axis[1] += ti * v;
        }
    }
    let n = t[0];
    let sxx = t[2] - t[1] * t[1] / n;
    let solve = |[sy, sty]: [f64; 2]| (sty - t[1] * sy / n) / sxx;
    (sxx > 0.0).then(|| (solve(sums[0]), solve(sums[1])))
}
