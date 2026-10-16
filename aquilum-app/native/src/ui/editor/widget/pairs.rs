//! Автозакрытие скобок и кавычек, обрамление выделения.

use super::TextEditor;
use crate::ui::editor::history::Kind;

pub fn pair_close(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        '«' => Some('»'),
        _ => None,
    }
}

pub fn is_closing(c: char) -> bool {
    matches!(c, ')' | ']' | '}' | '"' | '\'' | '`' | '»')
}

pub fn is_matching_pair(open: char, close: char) -> bool {
    pair_close(open) == Some(close)
}

impl TextEditor {
    pub(super) fn handle_pair_typing(&mut self, text: &str) -> bool {
        let mut chars = text.chars();
        let Some(c) = chars.next() else { return false };
        if chars.next().is_some() {
            return false;
        }

        let range = self.selection();
        if !range.is_empty() {
            if let Some(close) = pair_close(c) {
                let start = range.start + c.len_utf8();
                self.dispatch(vec![(range.start..range.start, c.to_string()), (range.end..range.end, close.to_string())], Some((start, start + range.len())), Kind::Typing);
                return true;
            }
            return false;
        }

        if is_closing(c) {
            let next_char = self.doc.after(self.focus).chars().next();
            if next_char == Some(c) {
                let next_pos = self.focus + c.len_utf8();
                (self.anchor, self.focus) = (next_pos, next_pos);
                return true;
            }
        }

        if let Some(close) = pair_close(c) {
            let prev_char = self.doc.before(self.focus).chars().next_back();
            if (c == '\'' || c == '"' || c == '`') && prev_char.is_some_and(char::is_alphanumeric) {
                return false;
            }
            let inside = self.focus + c.len_utf8();
            self.replace_and_select(self.focus..self.focus, &format!("{c}{close}"), Kind::Typing, (inside, inside));
            return true;
        }

        false
    }

    pub(super) fn handle_pair_backspace(&mut self) -> bool {
        if !self.selection().is_empty() || self.focus == 0 {
            return false;
        }
        let prev = self.doc.before(self.focus).chars().next_back();
        let next = self.doc.after(self.focus).chars().next();
        if let (Some(p), Some(n)) = (prev, next)
            && is_matching_pair(p, n)
        {
            let from = self.focus - p.len_utf8();
            let to = self.focus + n.len_utf8();
            self.replace(from..to, "", Kind::Deleting);
            return true;
        }
        false
    }
}
