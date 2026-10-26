use std::collections::VecDeque;
use std::time::{Duration, Instant};

const TOUCHPAD_HORIZON: Duration = Duration::from_millis(40);
const HORIZON: Duration = Duration::from_millis(100);
const STOPPED: Duration = Duration::from_millis(40);
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
        Trail { horizon: HORIZON, ..Trail::default() }
    }

    pub fn push(&mut self, at: Instant, x: f64, y: f64) {
        if self.samples.back().is_some_and(|&(t, _, _)| at.saturating_duration_since(t) > STOPPED) {
            self.samples.clear();
        }
        if self.samples.back().is_some_and(|&(t, _, _)| t == at) {
            self.samples.pop_back();
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
        slope(self.samples.iter().map(|&(t, x, y)| (age(t), x, y)), self.samples.len() > 2)
    }
}

pub(super) fn slope(points: impl Iterator<Item = (f64, f64, f64)>, quadratic: bool) -> Option<(f64, f64)> {
    let mut t = [0.0; 5];
    let mut sums = [[0.0; 3]; 2];
    for (ti, x, y) in points {
        for (k, sum) in t.iter_mut().enumerate() {
            *sum += ti.powi(k as i32);
        }
        for (axis, v) in sums.iter_mut().zip([x, y]) {
            axis[0] += v;
            axis[1] += ti * v;
            axis[2] += ti * ti * v;
        }
    }
    let n = t[0];
    let sxx = t[2] - t[1] * t[1] / n;
    let sxx2 = t[3] - t[1] * t[2] / n;
    let sx2x2 = t[4] - t[2] * t[2] / n;
    let denominator = if quadratic { sxx * sx2x2 - sxx2 * sxx2 } else { sxx };
    if denominator == 0.0 {
        return None;
    }
    let solve = |[sy, sty, st2y]: [f64; 3]| {
        let sxy = sty - t[1] * sy / n;
        let sx2y = st2y - t[2] * sy / n;
        if quadratic { (sxy * sx2x2 - sx2y * sxx2) / denominator } else { sxy / denominator }
    };
    Some((solve(sums[0]), solve(sums[1])))
}
