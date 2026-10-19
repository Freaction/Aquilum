use std::collections::VecDeque;
use std::time::{Duration, Instant};

const HORIZON: Duration = Duration::from_millis(40);
const BEND_MS: f64 = 150.0;
const POWER: f64 = 1.32;
const FRICTION_PER_MS: f64 = 0.165;

#[derive(Default)]
pub struct Trail(VecDeque<(Instant, f64, f64)>);

impl Trail {
    pub fn push(&mut self, at: Instant, x: f64, y: f64) {
        self.0.push_back((at, x, y));
        while self.0.front().is_some_and(|&(t, _, _)| at.saturating_duration_since(t) > HORIZON) {
            self.0.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn release(&mut self, scale: f64) -> Option<Fling> {
        let mut samples = std::mem::take(&mut self.0);
        let (last, x, y) = samples.pop_back()?;
        let &(from, px, py) = samples.back()?;
        let n = samples.len() as f64;
        let ms = |t: Instant| -from.saturating_duration_since(t).as_secs_f64() * 1000.0;
        let mean = samples.iter().fold((0.0, 0.0, 0.0), |m, &(t, x, y)| (m.0 + ms(t) / n, m.1 + x / n, m.2 + y / n));
        let spread: f64 = samples.iter().map(|&(t, _, _)| (ms(t) - mean.0).powi(2)).sum();
        if spread == 0.0 {
            return None;
        }
        let slope = |pick: fn(&(Instant, f64, f64)) -> f64, average: f64| samples.iter().map(|s| (ms(s.0) - mean.0) * (pick(s) - average)).sum::<f64>() / spread;
        Fling::new(from, last, (slope(|s| s.1, mean.1), slope(|s| s.2, mean.2)), (x - px, y - py), scale)
    }
}

pub struct Fling {
    start: Instant,
    speed: f64,
    direction: (f64, f64),
    friction: f64,
    end: f64,
    covered: f64,
}

impl Fling {
    fn new(from: Instant, last: Instant, (vx, vy): (f64, f64), (dx, dy): (f64, f64), scale: f64) -> Option<Self> {
        let measured = vx.hypot(vy);
        let friction = FRICTION_PER_MS * scale;
        if measured <= friction {
            return None;
        }
        let lift = ((dx * vx + dy * vy) / measured / measured).clamp(0.0, last.saturating_duration_since(from).as_secs_f64() * 1000.0);
        let start = from + Duration::from_secs_f64(lift / 1000.0);
        let speed = measured + friction;
        let end = BEND_MS * ((speed / friction).powf(1.0 / POWER) - 1.0);
        Some(Fling { start, speed, direction: (vx / measured, vy / measured), friction, end, covered: 0.0 })
    }

    fn distance(&self, ms: f64) -> f64 {
        let ms = ms.clamp(0.0, self.end);
        self.speed * BEND_MS / (POWER - 1.0) * (1.0 - (1.0 + ms / BEND_MS).powf(1.0 - POWER)) - self.friction * ms
    }

    pub fn advance(&mut self, at: Instant) -> Option<(f64, f64)> {
        let ms = at.saturating_duration_since(self.start).as_secs_f64() * 1000.0;
        let distance = self.distance(ms);
        let step = distance - self.covered;
        self.covered = distance;
        (ms < self.end || step > 0.0).then_some((self.direction.0 * step, self.direction.1 * step))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = Duration::from_micros(8333);
    const SCALE: f64 = 1.75;

    fn released(step: f64, frames: u32) -> (Fling, Instant) {
        let start = Instant::now();
        let mut trail = Trail::default();
        for k in 0..frames {
            trail.push(start + FRAME * k, 0.0, step * f64::from(k));
        }
        (trail.release(SCALE).expect("инерция после движения"), start + FRAME * (frames - 1))
    }

    fn steps(mut fling: Fling, from: Instant, moved: f64) -> Vec<f64> {
        let mut steps: Vec<f64> = (0..1000).map_while(|k| fling.advance(from + FRAME * k)).map(|(_, y)| y).collect();
        steps[0] += moved;
        steps
    }

    #[test]
    fn continues_at_the_speed_of_the_fingers() {
        let (fling, last) = released(90.0, 12);
        let steps = steps(fling, last, 90.0);
        for step in &steps[..2] {
            assert!((step - 90.0).abs() < 90.0 * 0.05, "шаги отрыва и инерции {:?} вместо 90", &steps[..2]);
        }
    }

    fn recorded(steps: &[f64]) -> (Fling, Instant, f64) {
        let start = Instant::now();
        let mut trail = Trail::default();
        let mut at = 0.0;
        for (k, step) in steps.iter().enumerate() {
            at += step;
            trail.push(start + FRAME * k as u32, 0.0, at);
        }
        let mut last: Vec<f64> = steps[steps.len() - 5..steps.len() - 1].to_vec();
        last.sort_by(f64::total_cmp);
        let reference = (last[1] + last[2]) / 2.0;
        (trail.release(SCALE).expect("инерция после движения"), start + FRAME * (steps.len() as u32 - 1), reference)
    }

    #[test]
    fn a_release_in_the_middle_of_a_frame_does_not_stall() {
        let mut flick: Vec<f64> = vec![90.0; 11];
        flick.push(22.0);
        let (fling, last, reference) = recorded(&flick);
        let steps = steps(fling, last, 22.0);
        assert!(steps[..3].iter().all(|step| (0.85..=1.25).contains(&(step / reference))), "провал в кадре отрыва: {:?}", &steps[..3]);
    }

    #[test]
    fn real_flicks_do_not_brake_at_the_moment_of_release() {
        let flicks: [&[f64]; 4] = [
            &[30.0, 37.0, 47.0, 58.0, 67.0, 74.0, 80.0, 89.0],
            &[24.0, 32.0, 43.0, 52.0, 70.0, 77.0, 80.0, 86.0],
            &[46.0, 44.0, 50.0, 24.0, 88.0, 106.0, 100.0, 93.0],
            &[52.0, 75.0, 80.0, 94.0, 111.0, 120.0, 105.0, 45.0],
        ];
        for flick in flicks {
            let (fling, last, reference) = recorded(flick);
            let steps = steps(fling, last, flick[flick.len() - 1]);
            assert!((0.85..=1.25).contains(&(steps[1] / reference)), "первый шаг инерции {:.0} при скорости пальцев {reference:.0}: {flick:?}", steps[1]);
            assert!(steps[0] >= 0.85 * steps[1].min(reference), "провал в кадре отрыва {:.0}: {flick:?}", steps[0]);
        }
    }

    #[test]
    fn slows_down_smoothly_to_a_stop() {
        let (fling, last) = released(100.0, 12);
        let steps = steps(fling, last, 100.0);
        assert!(steps.windows(2).all(|w| w[1] <= w[0] && w[1] >= 0.0), "шаги не убывают плавно: {steps:?}");
        assert!(steps.windows(2).all(|w| w[0] - w[1] < 0.1 * w[0] + 0.5), "резкий скачок скорости: {steps:?}");
        assert!(*steps.last().unwrap() < 1.0, "остановка обрывом: {:?}", &steps[steps.len() - 3..]);
        let path: f64 = steps.iter().sum();
        assert!((2400.0..3100.0).contains(&path), "путь {path} не как у Windows (около 2700)");
    }

    #[test]
    fn resting_fingers_do_not_fling() {
        let start = Instant::now();
        let mut trail = Trail::default();
        for k in 0..12 {
            trail.push(start + FRAME * k, 0.0, 500.0);
        }
        assert!(trail.release(SCALE).is_none());
    }

    #[test]
    fn old_movement_is_forgotten() {
        let start = Instant::now();
        let mut trail = Trail::default();
        for k in 0..6 {
            trail.push(start + FRAME * k, 0.0, 80.0 * f64::from(k));
        }
        for k in 6..24 {
            trail.push(start + FRAME * k, 0.0, 400.0);
        }
        assert!(trail.release(SCALE).is_none(), "пальцы остановились, а инерция взялась из старого движения");
    }
}
