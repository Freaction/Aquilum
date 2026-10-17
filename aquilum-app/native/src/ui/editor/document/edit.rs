use std::ops::Range;

use super::super::{Edit, TextAction};
use super::Document;

impl Document {
    pub fn report_edits(&mut self) {
        self.edits = Some(Vec::new());
    }

    pub fn changed(&mut self) -> TextAction {
        match self.edits.as_mut() {
            Some(edits) => TextAction::Edited(std::mem::take(edits)),
            None => TextAction::Changed(self.whole()),
        }
    }

    pub fn set_rope(&mut self, text: crop::Rope) {
        (self.anchor, self.editing, self.resized, self.uncropped, self.source) = (None, None, None, None, None);
        if let Some(edits) = self.edits.as_mut() {
            edits.clear();
        }
        let base = self.base;
        self.full = text;
        self.rescan();
        self.load(base, None);
    }

    pub fn report(&mut self, range: Range<usize>, insert: &str) {
        if let Some(edits) = self.edits.as_mut() {
            edits.push(Edit { range, inserted: insert.len() });
        }
    }

    pub fn replace(&mut self, range: Range<usize>, insert: &str) {
        let shift = |at: usize| super::super::change::shift(at, &range, insert.len());
        for at in [self.anchor.as_mut().map(|(at, _)| at), self.editing.as_mut().map(|(at, _)| at), self.resized.as_mut().map(|(at, _)| at), self.uncropped.as_mut(), self.source.as_mut()].into_iter().flatten() {
            *at = shift(*at);
        }
        let (base, end) = self.loaded_range();
        if range.start >= base && range.end <= end {
            self.splice(range, insert);
            return;
        }
        let delta = insert.len() as isize - range.len() as isize;
        let before = self.guess(range.clone());
        self.full.replace(range.clone(), insert);
        let after = self.guess(range.start..range.start + insert.len());
        if range.end < base {
            self.base = base.wrapping_add_signed(delta);
            self.above = (self.above + after - before).max(0.0);
        } else if range.start > end {
            self.below = (self.below + after - before).max(0.0);
        } else {
            self.load(range.start, None);
        }
        self.rescan_from(range.start);
    }

    fn splice(&mut self, range: Range<usize>, insert: &str) {
        let window = self.loaded_range();
        let exit = self.blocks.state(self.lines.count() - 1);
        let first = self.at_offset(range.start);
        let last = self.at_offset(range.end);
        let from = self.lines.start(first);
        let to = self.lines.end(last);
        self.text.replace_range(range.start - self.base..range.end - self.base, insert);
        self.full.replace(range.clone(), insert);
        let delta = insert.len() as isize - range.len() as isize;
        let new_end = to.wrapping_add_signed(delta);
        let count = self.lines.splice(first..=last, &self.text[from..new_end], delta);
        self.heights.splice(first..=last, count);
        self.blocks.splice(first..=last, count);
        self.laid.splice(first..=last, count);
        self.embeds.splice(first..=last, count);
        let remap = |i: usize, end: bool| match i {
            i if i < first || end && i == first => i,
            i if i > last + usize::from(end) => i - (last + 1) + first + count,
            _ => first + if end { count } else { 0 },
        };
        self.dirty = self.dirty.take().map(|d| remap(d.start, false)..remap(d.end, true));
        self.mark(first..first + count);
        self.classify_from(first.saturating_sub(1), first + count);
        let exit_changed = exit != self.blocks.state(self.lines.count() - 1) && self.end() < self.len();
        self.checkpoints_edited(window, delta, exit_changed);
    }
}
