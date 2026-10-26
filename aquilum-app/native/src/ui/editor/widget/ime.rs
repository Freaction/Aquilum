use super::*;

const AROUND: usize = 1900;

impl TextEditor {
    pub fn multiline(&self) -> bool {
        !self.single_line
    }

    pub fn surrounding(&self) -> (String, usize, usize) {
        if let Some((value, caret, anchor)) = self.table_cell() {
            let (range, caret, anchor) = excerpt(value.len(), caret, anchor, |at| value.floor_char_boundary(at));
            return (value[range].to_owned(), caret, anchor);
        }
        let (range, caret, anchor) = excerpt(self.doc.len(), self.focus, self.anchor, |at| self.doc.floor(at));
        (self.doc.copy(range), caret, anchor)
    }

    pub(super) fn ime(&mut self, ctx: &mut EventCtx<'_>, ime: &Ime) {
        let mut edited = false;
        let preedit = matches!(ime, Ime::Preedit(..));
        match ime {
            Ime::Enabled => {}
            Ime::Disabled => self.compose = None,
            Ime::Preedit(text, _) if text.is_empty() => self.compose = None,
            Ime::Preedit(text, cursor) => {
                if !self.selection().is_empty() {
                    self.insert("", Kind::Other);
                    edited = true;
                }
                self.compose = Some((text.clone(), *cursor));
            }
            Ime::Commit(text) => {
                self.compose = None;
                self.insert(text, Kind::Other);
                edited = true;
            }
            Ime::DeleteSurrounding { before_bytes, after_bytes } => {
                self.delete_surrounding(*before_bytes, *after_bytes);
                edited = true;
            }
        }
        if edited {
            self.update_suggest(ctx.widget_id());
        }
        self.doc.invalidate(self.focus);
        ctx.set_handled();
        if edited {
            self.changed(ctx);
        } else {
            ctx.request_layout();
            ctx.request_render();
        }
        let compose = self.compose.as_ref().map_or("", |(text, _)| text.as_str());
        if preedit && let Some(value) = self.doc.composed(self.focus, compose) {
            ctx.submit_action::<TextAction>(TextAction::Changed(value));
        }
    }

    fn delete_surrounding(&mut self, before: usize, after: usize) {
        let range = self.selection();
        let start = self.doc.floor(range.start.saturating_sub(before));
        let end = self.doc.floor((range.end + after).min(self.doc.len()));
        let edits = [start..range.start, range.end..end].into_iter().filter(|r| !r.is_empty()).map(|r| (r, String::new())).collect();
        self.dispatch(edits, None, Kind::Other);
    }
}

fn excerpt(len: usize, caret: usize, anchor: usize, floor: impl Fn(usize) -> usize) -> (Range<usize>, usize, usize) {
    let start = floor(caret.saturating_sub(AROUND));
    let end = floor((caret + AROUND).min(len));
    let local = |at: usize| at.clamp(start, end) - start;
    (start..end, local(caret), local(anchor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_around_the_caret_keeps_the_selection() {
        let mut editor = TextEditor::new("привет мир");
        (editor.anchor, editor.focus) = (12, 12);
        editor.delete_surrounding(4, 3);
        assert_eq!((editor.doc.whole().as_str(), editor.selection()), ("привир", 8..8));
        let mut editor = TextEditor::new("abcdef");
        (editor.anchor, editor.focus) = (2, 4);
        editor.delete_surrounding(1, 1);
        assert_eq!((editor.doc.whole().as_str(), editor.selection()), ("acdf", 1..3));
    }

    #[test]
    fn the_surrounding_text_is_a_window_around_the_caret() {
        let mut editor = TextEditor::new(&"я".repeat(3000));
        (editor.anchor, editor.focus) = (6000, 3000);
        let (text, cursor, anchor) = editor.surrounding();
        assert!(text.len() < 4000);
        assert_eq!((cursor, anchor), (1900, text.len()));
        assert!(text.is_char_boundary(cursor));
    }
}
