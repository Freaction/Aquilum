use std::ops::Range;

use super::Document;

impl Document {
    pub fn slice(&self, range: Range<usize>) -> &str {
        &self.text[range.start - self.base..range.end - self.base]
    }

    pub fn before(&self, at: usize) -> &str {
        &self.text[..at - self.base]
    }

    pub fn after(&self, at: usize) -> &str {
        &self.text[at - self.base..]
    }

    pub fn loaded(&self) -> (usize, &str) {
        (self.base, &self.text)
    }

    pub fn end(&self) -> usize {
        self.base + self.text.len()
    }

    pub fn loaded_range(&self) -> (usize, usize) {
        (self.base, self.end())
    }

    pub fn include(&mut self, at: usize) {
        if at < self.base || at > self.end() {
            self.load(at, None);
        }
    }

    pub fn find(&self, pattern: &str) -> Option<usize> {
        self.text.find(pattern).map(|at| self.base + at)
    }

    pub fn search(&self, find: impl Fn(&str) -> Vec<Range<usize>>) -> Vec<Range<usize>> {
        find(&self.text).into_iter().map(|found| self.base + found.start..self.base + found.end).collect()
    }
}
