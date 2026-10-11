use std::ops::Range;

use masonry::imaging::Painter;
use masonry::parley::Affinity;
use masonry::peniko::Color;
use unicode_segmentation::UnicodeSegmentation;

use super::{Paragraph, TextEditor};
use crate::ui::editor::history::Kind;

impl TextEditor {
    pub(super) fn clear_extra_cursors(&mut self) -> bool {
        if self.extra_cursors.is_empty() {
            return false;
        }
        self.extra_cursors.clear();
        true
    }

    pub(super) fn add_cursor_at(&mut self, offset: usize) {
        if !self.extra_cursors.iter().any(|&(a, f)| a == offset && f == offset) && (self.anchor != offset || self.focus != offset) {
            self.extra_cursors.push((offset, offset));
        }
    }

    pub(super) fn paint_extra_carets(&self, painter: &mut Painter<'_>, color: Color) {
        if !self.caret_visible {
            return;
        }
        for &(_, focus) in &self.extra_cursors {
            if focus <= self.doc.len() {
                let caret = self.caret_at(focus, 0, Affinity::Downstream);
                painter.fill(caret, color).draw();
            }
        }
    }

    pub(super) fn paint_extra_selections(&self, painter: &mut Painter<'_>, shown: &[Paragraph], color: Color) {
        for &(anchor, focus) in &self.extra_cursors {
            let start = anchor.min(focus);
            let end = anchor.max(focus);
            if start < end {
                self.paint_range(painter, shown, start..end, color);
            }
        }
    }

    pub(super) fn apply_multicursor_typing(&mut self, text: &str) -> bool {
        self.each_cursor(|editor, range| {
            editor.replace(range.clone(), text, Kind::Typing);
            range.start + text.len()
        })
    }

    pub(super) fn apply_multicursor_delete(&mut self, forward: bool) -> bool {
        self.each_cursor(|editor, range| {
            let text = editor.doc.text();
            let range = if !range.is_empty() {
                range
            } else if forward {
                range.start..text[range.start..].graphemes(true).next().map_or(range.start, |g| range.start + g.len())
            } else {
                text[..range.start].graphemes(true).next_back().map_or(range.start, |g| range.start - g.len())..range.start
            };
            if range.is_empty() {
                return range.start;
            }
            editor.replace(range.clone(), "", Kind::Deleting);
            range.start
        })
    }

    fn each_cursor(&mut self, mut edit: impl FnMut(&mut Self, Range<usize>) -> usize) -> bool {
        if self.extra_cursors.is_empty() {
            return false;
        }
        let primary = (self.anchor.min(self.focus), self.anchor.max(self.focus));
        let mut all: Vec<(usize, usize)> = std::mem::take(&mut self.extra_cursors).into_iter().map(|(a, f)| (a.min(f), a.max(f))).collect();
        all.push(primary);
        all.sort_unstable_by(|a, b| b.cmp(a));
        all.dedup();
        let mut placed: Vec<(usize, bool)> = Vec::with_capacity(all.len());
        for (start, end) in all {
            let before = self.doc.len();
            let at = edit(self, start..end);
            let delta = self.doc.len() as isize - before as isize;
            for (offset, _) in &mut placed {
                *offset = offset.saturating_add_signed(delta);
            }
            placed.push((at, (start, end) == primary));
        }
        let main = placed.iter().position(|&(_, primary)| primary).unwrap_or(placed.len() - 1);
        let (offset, _) = placed.swap_remove(main);
        (self.anchor, self.focus) = (offset, offset);
        self.extra_cursors = placed.into_iter().map(|(offset, _)| (offset, offset)).collect();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::TextEditor;

    fn cursors(editor: &TextEditor) -> (usize, Vec<usize>) {
        let mut extra: Vec<usize> = editor.extra_cursors.iter().map(|&(_, f)| f).collect();
        extra.sort_unstable();
        (editor.focus, extra)
    }

    #[test]
    fn every_cursor_keeps_its_place_after_typing_and_deleting() {
        let mut editor = TextEditor::new("abcdefghij");
        (editor.anchor, editor.focus) = (2, 2);
        editor.add_cursor_at(5);
        editor.add_cursor_at(8);
        editor.apply_multicursor_typing("XY");
        assert_eq!(editor.doc.text(), "abXYcdeXYfghXYij");
        assert_eq!(cursors(&editor), (4, vec![9, 14]));
        editor.apply_multicursor_delete(false);
        assert_eq!(editor.doc.text(), "abXcdeXfghXij");
        assert_eq!(cursors(&editor), (3, vec![7, 11]));
    }

    #[test]
    fn every_cursor_deletes_a_whole_grapheme() {
        let text = "a👍🏽b e\u{301} c";
        let mut editor = TextEditor::new(text);
        let (thumb, accent) = ("a👍🏽".len(), "a👍🏽b e\u{301}".len());
        (editor.anchor, editor.focus) = (thumb, thumb);
        editor.add_cursor_at(accent);
        editor.apply_multicursor_delete(false);
        assert_eq!(editor.doc.text(), "ab  c");
    }
}
