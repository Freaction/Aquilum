use super::collapse::bezier;
use crate::ui::motion;

pub use motion::{EASE_OUT, FAST, NORMAL};

pub struct Intro {
    seconds: f64,
    elapsed: f64,
}

fn begin(seconds: f64) -> f64 {
    if motion::on() { 0.0 } else { seconds }
}

fn advanced(elapsed: f64, seconds: f64, interval: u64) -> f64 {
    if motion::on() { elapsed + interval as f64 / 1e9 } else { seconds }
}

impl Intro {
    pub fn new(seconds: f64) -> Self {
        Intro { seconds, elapsed: begin(seconds) }
    }

    pub fn restart(&mut self) {
        self.elapsed = begin(self.seconds);
    }

    pub fn reverse(&mut self) {
        self.elapsed = if motion::on() { (self.seconds - self.elapsed).max(0.0) } else { self.seconds };
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
        self.elapsed = advanced(self.elapsed, self.seconds, interval);
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
        self.elapsed = begin(self.seconds);
        true
    }

    pub fn advance(&mut self, interval: u64) -> bool {
        self.elapsed = advanced(self.elapsed, self.seconds, interval);
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

    #[test]
    fn turning_motion_on_does_not_rewind_what_finished_without_it() {
        motion::set(false);
        let (mut shown, mut faded) = (Intro::new(NORMAL), Intro::new(NORMAL));
        shown.advance(8_000_000);
        faded.reverse();
        let mut tween = Tween::new(0.0, FAST, EASE_OUT);
        tween.set(10.0);
        motion::set(true);
        assert!(!shown.running() && shown.value() == 1.0, "окно, открытое без анимаций, стало прозрачным");
        assert!(!faded.running() && faded.reversed() == 0.0);
        assert!(!tween.advance(0) && tween.value() == 10.0);
    }
}
