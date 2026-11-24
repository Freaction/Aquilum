use std::collections::VecDeque;
use std::time::{Duration, Instant};

const HORIZON: Duration = Duration::from_millis(40);
const HISTORY: usize = 20;

#[derive(Default)]
pub struct Trail {
    pub(super) samples: VecDeque<(Instant, f64, f64)>,
}

impl Trail {
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, at: Instant, x: f64, y: f64) {
        if let Some(&(t, _, _)) = self.samples.back() {
            if at.saturating_duration_since(t) < crate::ui::keymap::gesture::FLING_SAMPLE_GAP {
                self.samples.pop_back();
                self.samples.push_back((t, x, y));
                return;
            }
        }
        self.samples.push_back((at, x, y));
        while self.samples.len() > HISTORY || self.samples.front().is_some_and(|&(t, _, _)| at.saturating_duration_since(t) > HORIZON) {
            self.samples.pop_front();
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.samples.len()
    }

    pub(crate) fn fresh(&self, at: Instant) -> bool {
        self.samples.back().is_some_and(|&(newest, _, _)| at.saturating_duration_since(newest) <= HORIZON)
    }

    pub fn velocity(&self, at: Instant) -> Option<(f64, f64)> {
        if !self.fresh(at) {
            return None;
        }
        let &(newest, _, _) = self.samples.back()?;
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
