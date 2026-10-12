use super::*;
use crate::ui::editor::markdown::{self, Mark};
use aquilum_core::markdown::line;

impl TextEditor {
    pub(super) fn toggle_mark(&mut self, mark: Mark) -> bool {
        let range = self.selection();
        let text = self.doc.text();
        let (line_start, line_end) = (line::bounds(text, range.start).start, line::bounds(text, range.end).end);
        let local = range.start - line_start..range.end - line_start;
        let (outer, inner, selection) = match markdown::enclosing(&text[line_start..line_end], mark, local.clone()) {
            Some((whole, inside)) => {
                let (open, close) = (inside.start - whole.start, whole.end - inside.end);
                let shift = |p: usize| match p {
                    p if p <= whole.start => p,
                    p if p < inside.start => whole.start,
                    p if p <= inside.end => p - open,
                    p if p < whole.end => inside.end - open,
                    p => p - open - close,
                };
                let selection = (line_start + shift(local.start), line_start + shift(local.end));
                (line_start + whole.start..line_start + whole.end, text[line_start + inside.start..line_start + inside.end].to_owned(), selection)
            }
            None if range.is_empty() => return false,
            None => {
                let marker = mark.marker();
                (range.clone(), format!("{marker}{}{marker}", &text[range.clone()]), (range.start + marker.len(), range.end + marker.len()))
            }
        };
        let forward = self.anchor <= self.focus;
        self.replace(outer, &inner, Kind::Other);
        (self.anchor, self.focus) = if forward { selection } else { (selection.1, selection.0) };
        true
    }
}
