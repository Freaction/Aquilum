use std::ops::Range;

mod text;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    Select { anchor: usize, cursor: usize },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppState {
    pub text: String,
    pub anchor: usize,
    pub cursor: usize,
    pub preedit: Option<(String, usize)>,
}

#[derive(Debug, Default)]
pub struct Sync {
    app: AppState,
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

impl AppState {
    fn range(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }

    pub fn to_ime(&self) -> ImeState {
        let Some((preedit, at)) = &self.preedit else {
            return ImeState { text: self.text.clone(), anchor: self.anchor, cursor: self.cursor, compose: None };
        };
        let mut text = self.text.clone();
        text.insert_str(self.cursor, preedit);
        let caret = self.cursor + at;
        ImeState { text, anchor: caret, cursor: caret, compose: Some(self.cursor..self.cursor + preedit.len()) }
    }
}

impl Sync {
    pub fn app_changed(&mut self, app: AppState) -> bool {
        let changed = self.app != app;
        self.app = app;
        changed
    }

    pub fn ime_changed(&mut self, ime: &ImeState) -> Vec<Edit> {
        let target = self.target(ime);
        let mut edits = Vec::new();
        if target.text != self.app.text {
            self.replace(&target, &mut edits);
        }
        if target.preedit.is_none() && (self.app.anchor, self.app.cursor) != (target.anchor, target.cursor) {
            edits.push(Edit::Select { anchor: target.anchor, cursor: target.cursor });
            (self.app.anchor, self.app.cursor) = (target.anchor, target.cursor);
        }
        if target.preedit != self.app.preedit {
            let (text, at) = target.preedit.clone().unwrap_or_default();
            let cursor = (!text.is_empty()).then_some((at, at));
            edits.push(Edit::Preedit(text, cursor));
            self.app.preedit = target.preedit;
        }
        edits
    }

    fn target(&self, ime: &ImeState) -> AppState {
        let region = ime.compose.clone().filter(|region| ime.anchor == ime.cursor && (region.start..=region.end).contains(&ime.cursor));
        let Some(region) = region else {
            return AppState { text: ime.text.clone(), anchor: ime.anchor, cursor: ime.cursor, preedit: None };
        };
        let mut text = ime.text.clone();
        let preedit = text.drain(region.clone()).collect();
        AppState { text, anchor: region.start, cursor: region.start, preedit: Some((preedit, ime.cursor - region.start)) }
    }

    fn replace(&mut self, target: &AppState, edits: &mut Vec<Edit>) {
        let old = self.app.range();
        let (start, end, inserted) = text::changed(&self.app.text, &old, &target.text, &target.range());
        let (before, after) = (old.start - start, end - old.end);
        if self.app.preedit.take().is_some() {
            edits.push(Edit::Preedit(String::new(), None));
        }
        if before > 0 || after > 0 {
            edits.push(Edit::DeleteSurrounding { before, after });
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
        self.app = AppState { text: target.text.clone(), anchor: caret, cursor: caret, preedit: None };
    }
}
