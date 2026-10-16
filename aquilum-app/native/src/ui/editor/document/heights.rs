use std::ops::RangeInclusive;

use super::Fold;

pub(super) struct Heights {
    values: Vec<f32>,
    tree: Vec<f64>,
    text: Vec<f32>,
    folds: Vec<Fold>,
}

fn splice_column<T: Copy>(column: &mut Vec<T>, start: usize, end: usize, count: usize, fresh: T) {
    let kept = (end + 1 - start).min(count);
    column.splice(start + kept..end + 1, std::iter::repeat_n(fresh, count - kept));
}

impl Heights {
    pub(super) fn new(len: usize) -> Self {
        Heights { values: vec![0.0; len], tree: vec![0.0; len + 1], text: vec![0.0; len], folds: vec![Fold::None; len] }
    }

    pub(super) fn get(&self, i: usize) -> f64 {
        f64::from(self.values[i])
    }

    pub(super) fn text(&self, i: usize) -> f64 {
        f64::from(self.text[i])
    }

    pub(super) fn set_text(&mut self, i: usize, height: f64) -> bool {
        let height = height as f32;
        std::mem::replace(&mut self.text[i], height) != height
    }

    pub(super) fn fold(&self, i: usize) -> Fold {
        self.folds[i]
    }

    pub(super) fn lift(&self, i: usize) -> f64 {
        if self.folds[i] == Fold::Above { self.get(i) - self.text(i) } else { 0.0 }
    }

    pub(super) fn set(&mut self, i: usize, height: f64, fold: Fold) {
        self.folds[i] = fold;
        let delta = f64::from(height as f32) - f64::from(self.values[i]);
        self.values[i] = height as f32;
        let mut k = i + 1;
        while k < self.tree.len() {
            self.tree[k] += delta;
            k += k & k.wrapping_neg();
        }
    }

    pub(super) fn assign(&mut self, i: usize, height: f64, fold: Fold) {
        self.folds[i] = fold;
        self.values[i] = height as f32;
    }

    pub(super) fn rebuild(&mut self) {
        let n = self.values.len();
        self.tree.clear();
        self.tree.resize(n + 1, 0.0);
        for i in 1..=n {
            self.tree[i] += f64::from(self.values[i - 1]);
            let parent = i + (i & i.wrapping_neg());
            if parent <= n {
                self.tree[parent] += self.tree[i];
            }
        }
    }

    pub(super) fn splice(&mut self, range: RangeInclusive<usize>, count: usize) {
        let (start, end) = (*range.start(), *range.end());
        if end + 1 - start == count {
            return;
        }
        splice_column(&mut self.values, start, end, count, 0.0);
        splice_column(&mut self.text, start, end, count, 0.0);
        splice_column(&mut self.folds, start, end, count, Fold::None);
        self.rebuild();
    }

    pub(super) fn drop_front(&mut self, lines: usize) {
        self.values.drain(..lines);
        self.text.drain(..lines);
        self.folds.drain(..lines);
        self.rebuild();
    }

    pub(super) fn truncate(&mut self, keep: usize) {
        self.values.truncate(keep);
        self.text.truncate(keep);
        self.folds.truncate(keep);
        self.rebuild();
    }

    pub(super) fn extend(&mut self, count: usize) {
        self.values.extend(std::iter::repeat_n(0.0, count));
        self.text.extend(std::iter::repeat_n(0.0, count));
        self.folds.extend(std::iter::repeat_n(Fold::None, count));
        self.rebuild();
    }

    pub(super) fn prepend(&mut self, count: usize) {
        self.values.splice(0..0, std::iter::repeat_n(0.0, count));
        self.text.splice(0..0, std::iter::repeat_n(0.0, count));
        self.folds.splice(0..0, std::iter::repeat_n(Fold::None, count));
        self.rebuild();
    }

    pub(super) fn top(&self, i: usize) -> f64 {
        let (mut k, mut sum) = (i, 0.0);
        while k > 0 {
            sum += self.tree[k];
            k &= k - 1;
        }
        sum
    }

    pub(super) fn total(&self) -> f64 {
        self.top(self.values.len())
    }

    pub(super) fn near(&self, i: usize, (top, bottom): (f64, f64)) -> bool {
        let at = self.top(i);
        at + self.get(i) > top && at < bottom
    }

    pub(super) fn at(&self, y: f64) -> usize {
        let n = self.values.len();
        let (mut pos, mut rest) = (0, y);
        let mut step = n.checked_next_power_of_two().unwrap_or(0).max(1);
        while step > 0 {
            if pos + step <= n && self.tree[pos + step] <= rest {
                pos += step;
                rest -= self.tree[pos];
            }
            step /= 2;
        }
        pos.min(n.saturating_sub(1))
    }

    #[cfg(feature = "memory-probe")]
    pub(super) fn bytes(&self) -> usize {
        self.values.capacity() * 4 + self.tree.capacity() * 8 + self.text.capacity() * 4 + self.folds.capacity() * size_of::<Fold>()
    }
}

#[cfg(test)]
mod tests {
    use super::{Fold, Heights};

    fn heights(values: &[f64]) -> Heights {
        let mut h = Heights::new(values.len());
        for (i, v) in values.iter().enumerate() {
            h.assign(i, *v, Fold::None);
        }
        h.rebuild();
        h
    }

    #[test]
    fn tops_and_lookups_match_a_plain_running_sum() {
        let values: Vec<f64> = (0..1000).map(|i| [20.0, 0.0, 33.5, 0.0, 0.0, 120.0][i % 6]).collect();
        let mut h = heights(&values);
        let mut expected = values.clone();
        for (i, v) in [(5, 7.0), (6, 0.0), (999, 50.0), (0, 1.5)] {
            h.set(i, v, Fold::None);
            expected[i] = v;
        }
        let tops: Vec<f64> = expected.iter().scan(0.0, |s, v| Some(std::mem::replace(s, *s + v))).collect();
        for (i, top) in tops.iter().enumerate() {
            assert!((h.top(i) - top).abs() < 1e-6, "{i}");
        }
        assert!((h.total() - expected.iter().sum::<f64>()).abs() < 1e-6);
        for y in [-5.0, 0.0, 0.5, 21.0, 53.4, 53.5, 1234.0, 1e9] {
            let want = tops.partition_point(|t| *t <= y).saturating_sub(1);
            assert_eq!(h.at(y), want, "y = {y}");
        }
    }

    #[test]
    fn splicing_keeps_the_rest_in_place() {
        let mut h = heights(&[10.0, 20.0, 30.0, 40.0]);
        h.splice(1..=2, 3);
        h.set(1, 5.0, Fold::None);
        assert_eq!((h.top(4), h.total()), (45.0, 85.0));
        assert_eq!((h.at(14.9), h.at(45.0)), (1, 4));
        h.splice(1..=3, 1);
        assert_eq!((h.top(2), h.total()), (15.0, 55.0));
        h.splice(1..=1, 1);
        assert_eq!(h.total(), 55.0);
    }
}
