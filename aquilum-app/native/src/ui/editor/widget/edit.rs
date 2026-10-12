use super::*;

impl TextEditor {
    pub(super) fn replace(&mut self, range: Range<usize>, text: &str, kind: Kind) {
        let before = (self.anchor, self.focus);
        let removed = self.doc.text()[range.clone()].to_owned();
        self.doc.replace(range.clone(), text);
        self.anchor = range.start + text.len();
        self.focus = self.anchor;
        self.affinity = Affinity::Downstream;
        self.column = None;
        let edit = Edit { at: range.start, removed, inserted: text.to_owned(), before, after: (self.anchor, self.focus), kind, time: std::time::Instant::now() };
        self.history.record(edit);
        if self.doc.markdown.is_some() {
            self.renumber();
        }
        self.reveal_edited_image();
        self.refind();
    }

    fn reveal_edited_image(&mut self) {
        let p = self.doc.paragraph(self.doc.at_offset(self.focus));
        if matches!(p.block, crate::ui::editor::markdown::Block::Image) {
            self.doc.source = Some(p.start);
        }
    }

    fn renumber(&mut self) {
        if self.doc.paragraph(self.doc.at_offset(self.focus)).block.is_code() {
            return;
        }
        let Some(last) = self.history.last_mut() else { return };
        let Some((range, numbers)) = super::outline::renumber(self.doc.text(), last.at..last.at + last.inserted.len()) else { return };
        let text = self.doc.text();
        let (lo, hi) = (last.at.min(range.start), (last.at + last.inserted.len()).max(range.end));
        let mut original = text[lo..hi].to_owned();
        original.replace_range(last.at - lo..last.at - lo + last.inserted.len(), &last.removed);
        let mut updated = text[lo..hi].to_owned();
        updated.replace_range(range.start - lo..range.end - lo, &numbers);
        let shift = |p: usize| if p >= range.end { p + numbers.len() - range.len() } else { p.min(range.start + numbers.len()) };
        let selection = (shift(self.anchor), shift(self.focus));
        (last.at, last.removed, last.inserted, last.after) = (lo, original, updated, selection);
        self.doc.replace(range, &numbers);
        (self.anchor, self.focus) = selection;
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
        let edit = if redo { self.history.redo() } else { self.history.undo() };
        let Some(edit) = edit else { return false };
        let (from, to, selection) = if redo { (&edit.removed, &edit.inserted, edit.after) } else { (&edit.inserted, &edit.removed, edit.before) };
        self.doc.replace(edit.at..edit.at + from.len(), to);
        (self.anchor, self.focus) = selection;
        self.affinity = Affinity::Downstream;
        self.column = None;
        self.reveal_edited_image();
        self.refind();
        true
    }
}
