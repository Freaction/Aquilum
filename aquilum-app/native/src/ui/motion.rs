use std::cell::Cell;

pub use super::tokens::motion::*;

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(true) };
}

pub fn set(enabled: bool) {
    ENABLED.set(enabled);
}

pub fn on() -> bool {
    ENABLED.get()
}

pub fn progress(elapsed: f64, seconds: f64) -> f64 {
    if on() && seconds > 0.0 { (elapsed / seconds).min(1.0) } else { 1.0 }
}

pub fn running(elapsed: f64, seconds: f64) -> bool {
    on() && elapsed < seconds
}

pub fn step(passed: f64, seconds: f64) -> f64 {
    if on() && seconds > 0.0 { passed / seconds } else { 1.0 }
}

pub fn eased(fraction: f64) -> f64 {
    if on() { fraction } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tokens_come_from_the_design_system() {
        assert_eq!((FAST, MEDIUM, NORMAL, MORPH, SLOWER), (0.1, 0.15, 0.2, 0.45, 0.5));
        assert_eq!(EASE_OUT, (0.0, 0.0, 0.2, 1.0));
        assert_eq!(EASE_DEFAULT, (0.4, 0.0, 0.2, 1.0));
    }

    #[test]
    fn switching_motion_off_finishes_everything_at_once() {
        set(true);
        assert_eq!((progress(0.05, 0.1), running(0.05, 0.1), step(0.05, 0.1)), (0.5, true, 0.5));
        set(false);
        assert_eq!((progress(0.05, 0.1), running(0.05, 0.1), step(0.05, 0.1)), (1.0, false, 1.0));
        assert!(!running(0.0, 0.1) && step(0.0, 0.1) >= 1.0);
        set(true);
    }
}
