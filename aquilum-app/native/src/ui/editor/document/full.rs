use std::ops::Range;

use super::Document;

impl Document {
    pub fn len(&self) -> usize {
        self.full.byte_len()
    }

    pub(super) fn count(&self) -> usize {
        let len = self.full.byte_len();
        self.full.line_len() + usize::from(len == 0 || self.full.byte(len - 1) == b'\n')
    }

    pub(super) fn line_of(&self, byte: usize) -> usize {
        self.full.line_of_byte(self.floor(byte))
    }

    pub(super) fn line_start(&self, line: usize) -> usize {
        self.full.byte_of_line(line)
    }

    pub(super) fn line_end(&self, line: usize) -> usize {
        if line + 1 < self.count() { self.full.byte_of_line(line + 1) - 1 } else { self.full.byte_len() }
    }

    pub fn floor(&self, at: usize) -> usize {
        crate::rope::floor(&self.full, at)
    }

    pub fn whole(&self) -> String {
        self.full.to_string()
    }

    pub fn copy(&self, range: Range<usize>) -> String {
        self.full.byte_slice(range).to_string()
    }

    pub fn line_at(&self, at: usize) -> Range<usize> {
        let line = self.line_of(at);
        self.line_start(line)..self.line_end(line)
    }

    pub fn byte(&self, at: usize) -> u8 {
        self.full.byte(at)
    }

    pub fn next_grapheme(&self, at: usize) -> usize {
        self.full.byte_slice(at..).graphemes().next().map_or(0, |g| g.len())
    }

    pub fn previous_grapheme(&self, at: usize) -> usize {
        self.full.byte_slice(..at).graphemes().next_back().map_or(0, |g| g.len())
    }

    pub fn front(&self) -> usize {
        self.front
    }

    pub fn rope(&self) -> crop::Rope {
        self.full.clone()
    }
}
