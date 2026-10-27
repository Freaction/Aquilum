use std::collections::VecDeque;

use masonry::core::Ime;

use super::{AppState, Edit, ImeState, Message};

pub struct Keyboard {
    pub field: ImeState,
    session: i32,
    base: Option<usize>,
    sent: i32,
    last: Option<(ImeState, Option<usize>)>,
    pub outbox: VecDeque<Message>,
}

impl Keyboard {
    pub fn new(session: i32) -> Self {
        Keyboard { field: ImeState::default(), session, base: None, sent: 0, last: None, outbox: VecDeque::new() }
    }

    pub fn start(&mut self, session: i32) {
        (self.field, self.session, self.base, self.last) = (ImeState::default(), session, None, None);
    }

    fn send(&mut self, echo: bool) {
        let now = Some((self.field.clone(), self.base));
        if self.last != now {
            self.last = now;
            self.sent += 1;
            self.outbox.push_back(Message { session: self.session, seq: self.sent, echo, base: self.base, state: self.field.clone() });
        }
    }

    pub fn apply(&mut self, applied: i32, base: usize, state: ImeState) {
        if applied < self.sent || self.last == Some((state.clone(), Some(base))) {
            return;
        }
        (self.field, self.base) = (state, Some(base));
        self.send(true);
    }

    pub fn finish_composing(&mut self) {
        self.field.compose = None;
        self.send(false);
    }

    pub fn write(&mut self, text: &str, composing: bool) {
        let field = &mut self.field;
        let range = field.compose.take().unwrap_or(field.anchor.min(field.cursor)..field.anchor.max(field.cursor));
        field.text.replace_range(range.clone(), text);
        let end = range.start + text.len();
        (field.anchor, field.cursor) = (end, end);
        field.compose = (composing && !text.is_empty()).then_some(range.start..end);
        self.send(false);
    }

    pub fn backspace(&mut self) {
        let caret = self.field.cursor;
        if let Some(c) = self.field.text[..caret].chars().next_back() {
            self.field.text.replace_range(caret - c.len_utf8()..caret, "");
            (self.field.anchor, self.field.cursor) = (caret - c.len_utf8(), caret - c.len_utf8());
        }
        self.send(false);
    }
}

pub struct App {
    state: AppState,
}

impl App {
    pub fn new(text: &str) -> Self {
        App { state: AppState { text: text.into(), anchor: text.len(), cursor: text.len(), preedit: None } }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn text(&self) -> &str {
        &self.state.text
    }

    fn replace(&mut self, with: &str) {
        let range = self.state.range();
        self.state.text.replace_range(range.clone(), with);
        let caret = range.start + with.len();
        (self.state.anchor, self.state.cursor) = (caret, caret);
    }

    pub fn apply(&mut self, edit: Edit) {
        match edit {
            Edit::Ime(Ime::DeleteSurrounding { before_bytes, after_bytes }) => {
                let range = self.state.range();
                let state = &mut self.state;
                state.text.replace_range(range.end..(range.end + after_bytes).min(state.text.len()), "");
                let from = range.start.saturating_sub(before_bytes);
                state.text.replace_range(from..range.start, "");
                (state.anchor, state.cursor) = (state.anchor - (range.start - from), state.cursor - (range.start - from));
            }
            Edit::Ime(Ime::Preedit(text, cursor)) => {
                self.state.preedit = None;
                if !text.is_empty() {
                    self.replace("");
                    let at = cursor.map_or(text.len(), |(at, _)| at);
                    self.state.preedit = Some((text, at));
                }
            }
            Edit::Ime(Ime::Commit(text)) => {
                self.state.preedit = None;
                self.replace(&text);
            }
            Edit::Ime(Ime::Enabled | Ime::Disabled) => {}
            Edit::Enter => {
                self.state.preedit = None;
                let line = self.state.text[..self.state.cursor].rfind('\n').map_or(0, |at| at + 1);
                let next = if self.state.text[line..].starts_with("- ") { "\n- " } else { "\n" };
                self.replace(next);
            }
            Edit::Select { anchor, cursor } => (self.state.anchor, self.state.cursor) = (anchor, cursor),
        }
    }
}
