use super::*;
use crate::ui::editor::change::{self, Change};
use crate::ui::editor::history::Step;

impl TextEditor {
    pub(super) fn replace(&mut self, range: Range<usize>, text: &str, kind: Kind) {
        let end = range.start + text.len();
        self.dispatch(vec![(range, text.to_owned())], Some((end, end)), kind);
    }

    pub(super) fn dispatch(&mut self, mut edits: Vec<(Range<usize>, String)>, selection: Option<(usize, usize)>, kind: Kind) {
        let Some(from) = edits.iter().map(|(range, _)| range.start).min() else { return };
        let until = edits.iter().map(|(range, _)| range.end).max().unwrap_or(from);
        let before = (self.anchor, self.focus);
        edits.sort_unstable_by_key(|(range, _)| std::cmp::Reverse(range.start));
        let mut changes: Vec<Change> = edits.into_iter().map(|(range, text)| self.splice(range, text)).collect();
        let mut selection = selection.unwrap_or_else(|| (change::map(before.0, &changes), change::map(before.1, &changes)));
        if self.doc.markdown.is_some() {
            for (range, number) in super::outline::renumbered(&self.doc, from..change::map(until, &changes)).into_iter().rev() {
                let change = self.splice(range, number);
                selection = (change.map(selection.0), change.map(selection.1));
                changes.push(change);
            }
        }
        self.place(selection, &changes);
        self.history.record(Step { changes, before, after: selection, kind, time: std::time::Instant::now() });
    }

    fn place(&mut self, selection: (usize, usize), changes: &[Change]) {
        (self.anchor, self.focus) = selection;
        self.affinity = Affinity::Downstream;
        self.column = None;
        self.refind(changes);
        self.reveal_edited_image();
    }

    fn splice(&mut self, range: Range<usize>, inserted: String) -> Change {
        self.doc.report(range.clone(), &inserted);
        self.apply_change(range, inserted)
    }

    pub(super) fn apply_change(&mut self, range: Range<usize>, inserted: String) -> Change {
        let removed = self.doc.copy(range.clone());
        self.doc.replace(range.clone(), &inserted);
        let change = Change { at: range.start, removed, inserted };
        let map = |at: usize| change.map(at);
        for (anchor, focus) in &mut self.extra_cursors {
            (*anchor, *focus) = (map(*anchor), map(*focus));
        }
        if let Some(session) = &mut self.table {
            session.start = map(session.start);
        }
        self.table_scrolls = self.table_scrolls.drain().map(|(start, x)| (map(start), x)).collect();
        self.hovered_table = self.hovered_table.map(map);
        self.hovered_link = self.hovered_link.map(|(start, hit)| (map(start), hit));
        self.unit = self.unit.take().map(|(count, range)| (count, map(range.start)..map(range.end)));
        self.external_reveal = self.external_reveal.take().map(|(range, at)| (map(range.start)..map(range.end), at));
        self.media.remap(map);
        change
    }

    fn reveal_edited_image(&mut self) {
        let p = self.doc.paragraph(self.doc.at_offset(self.focus));
        if matches!(p.block, crate::ui::editor::markdown::Block::Image) {
            self.doc.source = Some(p.start);
        }
    }

    pub(super) fn insert(&mut self, text: &str, kind: Kind) {
        let flat;
        let text = if self.single_line && text.contains(['\r', '\n']) {
            flat = text.replace("\r\n", " ").replace(['\r', '\n'], " ");
            flat.as_str()
        } else {
            text
        };
        self.replace(self.selection(), text, kind);
    }

    pub(super) fn delete(&mut self, forward: bool, word: bool) {
        let range = self.selection();
        if !range.is_empty() {
            self.replace(range, "", Kind::Other);
            return;
        }
        let (target, _) = self.step(forward, word);
        let range = self.focus.min(target)..self.focus.max(target);
        if !range.is_empty() {
            self.replace(range, "", Kind::Deleting);
        }
    }

    pub(super) fn undo(&mut self, redo: bool) -> bool {
        self.leave_table();
        let Some(step) = self.history.take(redo) else { return false };
        let applied: Vec<Change> = if redo {
            step.changes.into_iter().map(|c| self.splice(c.at..c.at + c.removed.len(), c.inserted)).collect()
        } else {
            step.changes.into_iter().rev().map(|c| self.splice(c.at..c.at + c.inserted.len(), c.removed)).collect()
        };
        self.place(if redo { step.after } else { step.before }, &applied);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cursor_follows_an_edit_made_at_another_one() {
        let mut editor = TextEditor::new("привет\nмир");
        editor.add_cursor_at(editor.doc.len());
        editor.replace(0..0, "\n", Kind::Other);
        assert_eq!(editor.extra_cursors, [(editor.doc.len(), editor.doc.len())]);
        editor.undo(false);
        assert_eq!(editor.extra_cursors, [(editor.doc.len(), editor.doc.len())]);
    }

    #[test]
    fn a_rewritten_block_keeps_the_caret_on_its_text() {
        let mut editor = TextEditor::new("![](a.png|300)\nтекст");
        let end = editor.doc.len();
        (editor.anchor, editor.focus) = (end, end);
        editor.dispatch(vec![(0..14, "![](a.png|1200)".into())], None, Kind::Block);
        assert_eq!((editor.anchor, editor.focus), (end + 1, end + 1));
        editor.undo(false);
        assert_eq!(editor.doc.whole(), "![](a.png|300)\nтекст");
    }

    #[test]
    fn typing_at_several_cursors_is_one_undo_step() {
        let mut editor = TextEditor::new("ab\ncd\nef");
        editor.add_cursor_at(3);
        editor.add_cursor_at(6);
        editor.apply_multicursor_typing("X");
        assert_eq!(editor.doc.whole(), "Xab\nXcd\nXef");
        editor.undo(false);
        assert_eq!(editor.doc.whole(), "ab\ncd\nef");
    }
}
