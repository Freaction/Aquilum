use super::*;

const AROUND: usize = 1900;
const MARGIN: usize = 400;

impl TextEditor {
    pub fn multiline(&self) -> bool {
        !self.single_line
    }

    pub fn surrounding(&self, base: Option<usize>) -> (usize, String, usize, usize) {
        if let Some((value, session)) = self.table_cell() {
            let (text, caret, anchor) = match &session.compose {
                Some(range) => ([&value[..range.start], &value[range.end..]].concat(), range.start, range.start),
                None => (value.to_owned(), session.caret, session.anchor),
            };
            let (range, caret, anchor) = excerpt(text.len(), caret, anchor, None, |at| text.floor_char_boundary(at));
            return (range.start, text[range].to_owned(), caret, anchor);
        }
        let (range, caret, anchor) = excerpt(self.doc.len(), self.focus, self.anchor, base, |at| self.doc.floor(at));
        (range.start, self.doc.copy(range), caret, anchor)
    }

    #[cfg(target_os = "android")]
    pub fn composing(&self) -> Option<(String, usize)> {
        if let Some((value, session)) = self.table_cell() {
            return session.compose.clone().map(|range| (value[range.clone()].to_owned(), session.caret.saturating_sub(range.start)));
        }
        self.compose.as_ref().map(|(text, cursor)| (text.clone(), cursor.map_or(text.len(), |(at, _)| at)))
    }

    pub(super) fn ime(&mut self, ctx: &mut EventCtx<'_>, ime: &Ime) {
        let mut edited = false;
        let preedit = matches!(ime, Ime::Preedit(..));
        match ime {
            Ime::Enabled => {}
            Ime::Disabled => self.finish_compose(ctx),
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
        self.follow_caret |= edited || preedit;
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

    pub(super) fn finish_compose(&mut self, ctx: &mut EventCtx<'_>) {
        if let Some(session) = &mut self.table {
            session.compose = None;
        }
        if let Some((text, _)) = self.compose.take() {
            self.insert(&text, Kind::Other);
            self.changed(ctx);
        }
    }

    #[cfg(target_os = "android")]
    pub fn ime_select(this: &mut WidgetMut<'_, Self>, anchor: usize, cursor: usize) {
        if !this.widget.table_select(anchor, cursor) {
            Self::select_byte_range(this, anchor, cursor);
        }
        Self::keep_caret(this);
    }

    fn delete_surrounding(&mut self, before: usize, after: usize) {
        let range = self.selection();
        let start = self.doc.floor(range.start.saturating_sub(before));
        let end = self.doc.floor((range.end + after).min(self.doc.len()));
        let edits = [start..range.start, range.end..end].into_iter().filter(|r| !r.is_empty()).map(|r| (r, String::new())).collect();
        self.dispatch(edits, None, Kind::Other);
    }
}

fn excerpt(len: usize, caret: usize, anchor: usize, base: Option<usize>, floor: impl Fn(usize) -> usize) -> (Range<usize>, usize, usize) {
    let fits = |base: usize| base <= caret && (base == 0 || caret >= base + MARGIN) && (caret + MARGIN <= base + 2 * AROUND || base + 2 * AROUND >= len);
    let start = floor(base.filter(|&base| fits(base)).unwrap_or(caret.saturating_sub(AROUND)));
    let end = floor((start + 2 * AROUND).min(len));
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
        let (base, text, cursor, anchor) = editor.surrounding(None);
        assert!(text.len() < 4000);
        assert_eq!((base, cursor, anchor), (1100, 1900, text.len()));
        assert!(text.is_char_boundary(cursor));
    }

    #[test]
    fn the_window_stays_put_until_the_caret_nears_its_edge() {
        let mut editor = TextEditor::new(&"ab".repeat(5000));
        (editor.anchor, editor.focus) = (3000, 3000);
        let (base, ..) = editor.surrounding(None);
        (editor.anchor, editor.focus) = (3500, 3500);
        assert_eq!(editor.surrounding(Some(base)).0, base, "окно сдвинулось вместе с кареткой");
        (editor.anchor, editor.focus) = (base + 2 * AROUND - 100, base + 2 * AROUND - 100);
        assert_ne!(editor.surrounding(Some(base)).0, base, "каретка у края, а окно не переехало");
    }
}
