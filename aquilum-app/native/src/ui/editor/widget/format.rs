use super::*;
use crate::ui::editor::markdown::{self, Mark};
use aquilum_core::markdown::line;

impl TextEditor {
    pub(super) fn toggle_mark(&mut self, mark: Mark) -> bool {
        let range = self.selection();
        let text = self.doc.text();
        let (line_start, line_end) = (line::bounds(text, range.start).start, line::bounds(text, range.end).end);
        let local = range.start - line_start..range.end - line_start;
        match markdown::enclosing(&text[line_start..line_end], mark, local) {
            Some((whole, inside)) => {
                let (whole, inside) = (line_start + whole.start..line_start + whole.end, line_start + inside.start..line_start + inside.end);
                self.dispatch(vec![(whole.start..inside.start, String::new()), (inside.end..whole.end, String::new())], None, Kind::Other);
            }
            None if range.is_empty() => return false,
            None => {
                let marker = mark.marker();
                let shift = |at: usize| at + marker.len();
                self.dispatch(vec![(range.start..range.start, marker.to_owned()), (range.end..range.end, marker.to_owned())], Some((shift(self.anchor), shift(self.focus))), Kind::Other);
            }
        }
        true
    }
}
