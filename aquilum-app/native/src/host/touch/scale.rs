use crate::ui::keymap::gesture::{MIN_SCALING_SPAN, SCALING_SPAN_SLOP};

#[derive(Default)]
pub(super) struct Scale {
    initial: f64,
    previous: f64,
    scaling: bool,
}

impl Scale {
    pub(super) fn regroup(&mut self, span: f64, dp: f64) {
        self.scaling &= span >= MIN_SCALING_SPAN * dp;
        self.initial = span;
        self.previous = span;
    }

    pub(super) fn moved(&mut self, span: f64, dp: f64) -> f64 {
        let min = MIN_SCALING_SPAN * dp;
        if self.scaling && span < min {
            self.scaling = false;
            self.initial = span;
        }
        if !self.scaling && span >= min && (span - self.initial).abs() > SCALING_SPAN_SLOP * dp {
            self.scaling = true;
            self.previous = span;
        }
        let step = if self.scaling { (span / self.previous).ln() } else { 0.0 };
        self.previous = span;
        step
    }
}
