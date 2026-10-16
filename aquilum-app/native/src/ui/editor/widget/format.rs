use super::*;
use crate::ui::editor::markdown::{self, Mark};

impl TextEditor {
    pub(super) fn toggle_mark(&mut self, mark: Mark) -> bool {
        let range = self.selection();
        let (line_start, line_end) = (self.doc.line_at(range.start).start, self.doc.line_at(range.end).end);
        let text = &self.doc.copy(line_start..line_end);
        let local = range.start - line_start..range.end - line_start;
        match markdown::enclosing(text, mark, local) {
            Some((whole, inside)) => {
                let (whole, inside) = (line_start + whole.start..line_start + whole.end, line_start + inside.start..line_start + inside.end);
                self.dispatch(vec![(whole.start..inside.start, String::new()), (inside.end..whole.end, String::new())], None, Kind::Other);
            }
            None => {
                let range = match range.is_empty() {
                    false => range,
                    true => match markdown::word_at(text, range.start - line_start) {
                        Some(word) => line_start + word.start..line_start + word.end,
                        None => return false,
                    },
                };
                let marker = mark.marker();
                let shift = |at: usize| at + marker.len();
                self.dispatch(vec![(range.start..range.start, marker.to_owned()), (range.end..range.end, marker.to_owned())], Some((shift(self.anchor), shift(self.focus))), Kind::Other);
            }
        }
        true
    }
}
