use std::collections::HashMap;
use std::ops::RangeInclusive;

pub(super) struct Sparse<T>(HashMap<usize, T>);

impl<T> Default for Sparse<T> {
    fn default() -> Self {
        Sparse(HashMap::new())
    }
}

impl<T> Sparse<T> {
    pub(super) fn get(&self, line: usize) -> Option<&T> {
        self.0.get(&line)
    }

    pub(super) fn get_mut(&mut self, line: usize) -> Option<&mut T> {
        self.0.get_mut(&line)
    }

    pub(super) fn insert(&mut self, line: usize, value: T) {
        self.0.insert(line, value);
    }

    pub(super) fn remove(&mut self, line: usize) {
        self.0.remove(&line);
    }

    pub(super) fn clear(&mut self) {
        self.0.clear();
    }

    pub(super) fn retain(&mut self, keep: impl Fn(usize) -> bool) {
        self.0.retain(|line, _| keep(*line));
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.0.iter().map(|(line, value)| (*line, value))
    }

    pub(super) fn shift(&mut self, by: usize) {
        self.0 = std::mem::take(&mut self.0).into_iter().map(|(line, value)| (line + by, value)).collect();
    }

    pub(super) fn splice(&mut self, lines: RangeInclusive<usize>, count: usize) {
        let (first, last) = (*lines.start(), *lines.end());
        self.0 = std::mem::take(&mut self.0)
            .into_iter()
            .filter(|(line, _)| !lines.contains(line))
            .map(|(line, value)| (if line > last { line - last - 1 + first + count } else { line }, value))
            .collect();
    }
}
