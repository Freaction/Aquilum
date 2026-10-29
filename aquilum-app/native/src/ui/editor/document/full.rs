use std::ops::Range;

use super::Document;
use crate::ui::editor::caret;

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

    pub fn erase(&self, at: usize, forward: bool, word: bool) -> Range<usize> {
        let line = self.line_at(at);
        let local = caret::erase(&self.copy(line.clone()), at - line.start, forward, word);
        match (local.is_empty(), forward) {
            (false, _) => line.start + local.start..line.start + local.end,
            (true, true) => at..(at + 1).min(self.len()),
            (true, false) => at.saturating_sub(1)..at,
        }
    }

    pub fn front(&self) -> usize {
        self.front
    }

    pub fn rope(&self) -> crop::Rope {
        self.full.clone()
    }
}
