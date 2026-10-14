use std::ops::Range;

use super::super::{Edit, TextAction};
use super::classify::States;
use super::paragraph::split;
use super::Document;

impl Document {
    pub fn report_edits(&mut self) {
        self.edits = Some(Vec::new());
    }

    pub fn changed(&mut self) -> TextAction {
        match self.edits.as_mut() {
            Some(edits) => TextAction::Edited(std::mem::take(edits)),
            None => TextAction::Changed(self.text.clone()),
        }
    }

    pub fn set_text(&mut self, text: &str) {
        (self.anchor, self.editing, self.resized, self.uncropped, self.source) = (None, None, None, None, None);
        self.placed = false;
        self.states = States::default();
        if let Some(edits) = self.edits.as_mut() {
            edits.clear();
        }
        self.text = text.to_owned();
        self.paragraphs = split(text, 0);
        self.laid = 0..0;
        self.held.clear();
        self.mark(0..self.paragraphs.len());
        self.classify();
    }

    pub fn replace(&mut self, range: Range<usize>, insert: &str) {
        let first = self.at_offset(range.start);
        let last = self.at_offset(range.end);
        let from = self.paragraphs[first].start;
        let to = self.paragraphs[last].end();
        self.text.replace_range(range.clone(), insert);
        let shift = |at: usize| super::super::change::shift(at, &range, insert.len());
        for at in [self.anchor.as_mut().map(|(at, _)| at), self.editing.as_mut().map(|(at, _)| at), self.resized.as_mut().map(|(at, _)| at), self.uncropped.as_mut(), self.source.as_mut()].into_iter().flatten() {
            *at = shift(*at);
        }
        if let Some(edits) = self.edits.as_mut() {
            edits.push(Edit { range: range.clone(), insert: insert.to_owned() });
        }
        let delta = insert.len() as isize - range.len() as isize;
        let new_end = (to as isize + delta) as usize;
        let fresh = split(&self.text[from..new_end], from);
        for p in &mut self.paragraphs[last + 1..] {
            p.start = (p.start as isize + delta) as usize;
        }
        let top = self.paragraphs[first].top;
        let mut fresh = fresh;
        for p in &mut fresh {
            p.top = top;
        }
        let count = fresh.len();
        self.paragraphs.splice(first..=last, fresh);
        let remap = |i: usize, end: bool| match i {
            i if i < first || end && i == first => i,
            i if i > last + usize::from(end) => i - (last + 1) + first + count,
            _ => first + if end { count } else { 0 },
        };
        self.dirty = self.dirty.take().map(|d| remap(d.start, false)..remap(d.end, true));
        self.laid = remap(self.laid.start, false)..remap(self.laid.end, true);
        for i in &mut self.held {
            *i = remap(*i, false);
        }
        self.mark(first..first + count);
        self.classify_from(first.saturating_sub(1), first + count);
    }
}
