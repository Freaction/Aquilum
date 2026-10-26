use std::ops::Range;

mod text;
#[cfg(test)]
mod tests;

const CONTEXT: usize = 1024;
const SPAN: usize = 8000;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImeState {
    pub text: String,
    pub anchor: usize,
    pub cursor: usize,
    pub compose: Option<Range<usize>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Edit {
    DeleteSurrounding { before: usize, after: usize },
    Preedit(String, Option<(usize, usize)>),
    Commit(String),
    Enter,
    Left(usize),
    Right(usize),
}

#[derive(Debug, Default)]
struct View {
    text: String,
    anchor: usize,
    cursor: usize,
    preedit: Option<(String, usize)>,
}

#[derive(Debug)]
pub struct Bridge {
    app: View,
    base: String,
    pushed: Option<String>,
    stale: bool,
}

impl ImeState {
    pub fn from_utf16(text: String, selection: (usize, usize), compose: Option<(usize, usize)>) -> Self {
        let at = |unit| text::byte_of(&text, unit);
        let compose = compose.map(|(a, b)| at(a.min(b))..at(a.max(b))).filter(|range| !range.is_empty());
        Self { anchor: at(selection.0), cursor: at(selection.1), compose, text }
    }

    pub fn utf16(&self) -> ((usize, usize), Option<(usize, usize)>) {
        let at = |byte| text::utf16_of(&self.text, byte);
        ((at(self.anchor), at(self.cursor)), self.compose.as_ref().map(|range| (at(range.start), at(range.end))))
    }
}

impl View {
    fn range(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }

    fn to_ime(&self) -> ImeState {
        let Some((preedit, at)) = &self.preedit else {
            return ImeState { text: self.text.clone(), anchor: self.anchor, cursor: self.cursor, compose: None };
        };
        let mut text = self.text.clone();
        text.insert_str(self.cursor, preedit);
        let caret = self.cursor + at;
        ImeState { text, anchor: caret, cursor: caret, compose: Some(self.cursor..self.cursor + preedit.len()) }
    }

    fn agrees(&self, report: &View) -> bool {
        let (own, other) = (self.range(), report.range());
        let (before, report_before) = (&self.text[..own.start], &report.text[..other.start]);
        let (after, report_after) = (&self.text[own.end..], &report.text[other.end..]);
        self.text.len() <= SPAN
            && (self.cursor < self.anchor) == (report.cursor < report.anchor)
            && self.text[own] == report.text[other]
            && (before.ends_with(report_before) || report_before.ends_with(before))
            && (after.starts_with(report_after) || report_after.starts_with(after))
            && before.len() >= report_before.len().min(CONTEXT)
            && after.len() >= report_after.len().min(CONTEXT)
    }
}

impl Bridge {
    pub fn new(base: String) -> Self {
        Self { app: View::default(), base, pushed: None, stale: true }
    }

    pub fn restart(&mut self) {
        self.stale = true;
    }

    pub fn app_changed(&mut self, text: String, cursor: usize, anchor: usize) {
        let report = View { text, anchor, cursor, preedit: self.app.preedit.clone() };
        if self.stale || !self.app.agrees(&report) {
            self.app = report;
            self.stale = true;
        }
    }

    pub fn take_push(&mut self) -> Option<ImeState> {
        if !std::mem::take(&mut self.stale) {
            return None;
        }
        let state = self.app.to_ime();
        self.pushed = Some(state.text.clone());
        Some(state)
    }

    pub fn ime_changed(&mut self, ime: &ImeState) -> Vec<Edit> {
        let stale = self.pushed.take().is_some_and(|pushed| {
            text::distance(&self.base, &ime.text) < text::distance(&pushed, &ime.text)
        });
        self.base.clone_from(&ime.text);
        if stale {
            return Vec::new();
        }
        let target = self.target(ime);
        let mut edits = Vec::new();
        if target.text != self.app.text {
            self.replace(&target, &mut edits);
        }
        self.step(&target, &mut edits);
        if target.preedit != self.app.preedit {
            let (text, at) = target.preedit.clone().unwrap_or_default();
            let cursor = (!text.is_empty()).then_some((at, at));
            edits.push(Edit::Preedit(text, cursor));
            self.app.preedit = target.preedit;
        }
        edits
    }

    fn target(&self, ime: &ImeState) -> View {
        let region = ime.compose.clone().filter(|region| {
            ime.anchor == ime.cursor
                && (region.start..=region.end).contains(&ime.cursor)
                && (self.app.preedit.is_some() || ime.text != self.app.text)
        });
        let Some(region) = region else {
            return View { text: ime.text.clone(), anchor: ime.anchor, cursor: ime.cursor, preedit: None };
        };
        let mut text = ime.text.clone();
        let preedit = text.drain(region.clone()).collect();
        View { text, anchor: region.start, cursor: region.start, preedit: Some((preedit, ime.cursor - region.start)) }
    }

    fn replace(&mut self, target: &View, edits: &mut Vec<Edit>) {
        let old = self.app.range();
        let (start, end, inserted) = text::changed(&self.app.text, &old, &target.text, &target.range());
        let (before, after) = (old.start - start, end - old.end);
        if before > 0 || after > 0 {
            edits.push(Edit::DeleteSurrounding { before, after });
        }
        if self.app.preedit.take().is_some() {
            edits.push(Edit::Preedit(String::new(), None));
        }
        let line = inserted.strip_suffix('\n').filter(|line| !line.contains('\n'));
        let commit = line.unwrap_or(inserted);
        if !commit.is_empty() || !old.is_empty() {
            edits.push(Edit::Commit(commit.to_owned()));
        }
        if line.is_some() {
            edits.push(Edit::Enter);
        }
        let caret = start + inserted.len();
        self.app = View { text: target.text.clone(), anchor: caret, cursor: caret, preedit: None };
    }

    fn step(&mut self, target: &View, edits: &mut Vec<Edit>) {
        let (from, to) = (self.app.cursor, target.cursor);
        if (self.app.anchor, from) == (target.anchor, to) {
            return;
        }
        if self.app.anchor != from || target.anchor != to || self.app.preedit.is_some() {
            self.stale = true;
            return;
        }
        let chars = |range: Range<usize>| self.app.text[range].chars().count();
        edits.push(if to < from { Edit::Left(chars(to..from)) } else { Edit::Right(chars(from..to)) });
        (self.app.anchor, self.app.cursor) = (to, to);
    }
}
