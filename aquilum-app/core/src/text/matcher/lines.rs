use super::Matcher;
use crate::text::cancel::Cancel;
use std::ops::Range;

pub struct Lines<'a> {
    matcher: &'a Matcher,
    cancel: &'a Cancel,
    offset: usize,
    found: Vec<Range<usize>>,
}

impl<'a> Lines<'a> {
    pub(super) fn new(matcher: &'a Matcher, cancel: &'a Cancel) -> Self {
        Self { matcher, cancel, offset: 0, found: Vec::new() }
    }

    pub fn feed(&mut self, line: &str) -> bool {
        let (base, limit, found) = (self.offset, self.matcher.limit(), &mut self.found);
        let more = found.len() < limit
            && self.matcher.each(line, |range| {
                found.push(base + range.start..base + range.end);
                found.len() < limit
            });
        self.offset += line.len() + 1;
        more && !self.cancel.is_cancelled()
    }

    pub fn finish(self) -> Vec<Range<usize>> {
        self.found
    }
}
