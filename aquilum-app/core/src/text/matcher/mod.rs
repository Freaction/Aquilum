mod fold;
mod lines;
#[cfg(test)]
mod tests;

pub use fold::{fold, Fold, Folded};
pub use lines::Lines;

use super::cancel::Cancel;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Substring,
    WordPrefix,
    WholeWord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub fold: Fold,
    pub mode: Mode,
    pub limit: usize,
}

pub const EXACT: Options = Options { fold: Fold::Exact, mode: Mode::Substring, limit: usize::MAX };
pub const IGNORE_CASE: Options = Options { fold: Fold::Lower, ..EXACT };

pub struct Matcher {
    needles: Vec<Vec<char>>,
    options: Options,
}

impl Matcher {
    pub fn new(query: &str, options: Options) -> Self {
        Self::any(&[query], options)
    }

    pub fn any<S: AsRef<str>>(queries: &[S], options: Options) -> Self {
        let mut needles: Vec<Vec<char>> = queries
            .iter()
            .map(|query| fold(query.as_ref(), options.fold))
            .filter(|needle| !needle.is_empty())
            .collect();
        needles.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
        needles.dedup();
        Self { needles, options }
    }

    pub fn is_empty(&self) -> bool {
        self.needles.is_empty()
    }

    pub fn limit(&self) -> usize {
        self.options.limit
    }

    pub fn each(&self, text: &str, mut found: impl FnMut(Range<usize>) -> bool) -> bool {
        if self.needles.is_empty() {
            return true;
        }
        let mut rest = Folded::new(text, self.options.fold);
        let mut before = None;
        while let Some((first, start, end)) = rest.next() {
            if self.starts_word(before) {
                if let Some((end, after, last)) = self.match_at(first, end, &rest) {
                    if !found(start..end) {
                        return false;
                    }
                    (rest, before) = (after, Some(last));
                    rest.skip_rest_of_char();
                    continue;
                }
            }
            before = Some(first);
        }
        true
    }

    pub fn find_all(&self, text: &str) -> Vec<Range<usize>> {
        let mut found = Vec::new();
        if self.options.limit > 0 {
            self.each(text, |range| {
                found.push(range);
                found.len() < self.options.limit
            });
        }
        found
    }

    pub fn first(&self, text: &str) -> Option<Range<usize>> {
        let mut first = None;
        self.each(text, |range| {
            first = Some(range);
            false
        });
        first
    }

    pub fn lines<'a>(&'a self, cancel: &'a Cancel) -> Lines<'a> {
        Lines::new(self, cancel)
    }

    fn starts_word(&self, before: Option<char>) -> bool {
        self.options.mode == Mode::Substring || before.is_none_or(|character| !character.is_alphanumeric())
    }

    fn match_at<'t>(&self, first: char, first_end: usize, rest: &Folded<'t>) -> Option<(usize, Folded<'t>, char)> {
        'needles: for needle in &self.needles {
            if needle[0] != first {
                continue;
            }
            let (mut probe, mut end, mut last) = (rest.clone(), first_end, first);
            for &wanted in &needle[1..] {
                match probe.next() {
                    Some((folded, _, folded_end)) if folded == wanted => (end, last) = (folded_end, folded),
                    _ => continue 'needles,
                }
            }
            let ends_word = || probe.clone().next().is_none_or(|(after, ..)| !after.is_alphanumeric());
            if self.options.mode != Mode::WholeWord || ends_word() {
                return Some((end, probe, last));
            }
        }
        None
    }
}
