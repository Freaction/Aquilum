use super::collapse::bezier;
use crate::ui::motion;

pub use motion::{EASE_DEFAULT as EASE_STANDARD, EASE_LINEAR as LINEAR, EASE_OUT, FAST, NORMAL};

pub struct Intro {
    seconds: f64,
    elapsed: f64,
}

impl Intro {
    pub fn new(seconds: f64) -> Self {
        Intro { seconds, elapsed: 0.0 }
    }

    pub fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    pub fn reverse(&mut self) {
        self.elapsed = (self.seconds - self.elapsed).max(0.0);
    }

    pub fn finish(&mut self) {
        self.elapsed = self.seconds;
    }

    pub fn value(&self) -> f64 {
        bezier(EASE_OUT, motion::progress(self.elapsed, self.seconds))
    }

    pub fn reversed(&self) -> f64 {
        bezier(EASE_OUT, 1.0 - motion::progress(self.elapsed, self.seconds))
    }

    pub fn running(&self) -> bool {
        motion::running(self.elapsed, self.seconds)
    }

    pub fn advance(&mut self, interval: u64) -> bool {
        self.elapsed += interval as f64 / 1e9;
        self.running()
    }
}

pub struct Tween {
    from: f64,
    to: f64,
    elapsed: f64,
    seconds: f64,
    curve: (f64, f64, f64, f64),
}

impl Tween {
    pub fn new(value: f64, seconds: f64, curve: (f64, f64, f64, f64)) -> Self {
        Tween { from: value, to: value, elapsed: seconds, seconds, curve }
    }

    pub fn value(&self) -> f64 {
        self.from + (self.to - self.from) * bezier(self.curve, motion::progress(self.elapsed, self.seconds))
    }

    pub fn set(&mut self, to: f64) -> bool {
        if to == self.to {
            return false;
        }
        self.from = self.value();
        self.to = to;
        self.elapsed = 0.0;
        true
    }

    pub fn advance(&mut self, interval: u64) -> bool {
        self.elapsed += interval as f64 / 1e9;
        motion::running(self.elapsed, self.seconds)
    }

    pub fn finish(&mut self) {
        self.elapsed = self.seconds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_and_tween_jump_to_the_end_when_motion_is_off() {
        motion::set(true);
        let mut intro = Intro::new(NORMAL);
        assert!(intro.running() && intro.value() < 1.0);
        let mut tween = Tween::new(0.0, FAST, EASE_OUT);
        assert!(tween.set(10.0) && tween.value() < 10.0);
        motion::set(false);
        assert!(!intro.running() && intro.value() == 1.0 && intro.reversed() == 0.0);
        assert!(!intro.advance(1_000_000));
        assert_eq!(tween.value(), 10.0);
        assert!(tween.set(0.0));
        assert!(!tween.advance(1) && tween.value() == 0.0);
        motion::set(true);
    }
}
