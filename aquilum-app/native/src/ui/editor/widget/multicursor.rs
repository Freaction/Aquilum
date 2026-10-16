use std::ops::Range;

use masonry::imaging::Painter;
use masonry::parley::Affinity;
use masonry::peniko::Color;

use super::TextEditor;
use crate::ui::editor::document::Document;
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

    pub(super) fn paint_extra_selections(&self, painter: &mut Painter<'_>, shown: std::ops::RangeInclusive<usize>, color: Color) {
        for &(anchor, focus) in &self.extra_cursors {
            let start = anchor.min(focus);
            let end = anchor.max(focus);
            if start < end {
                self.paint_range(painter, shown.clone(), start..end, color);
            }
        }
    }

    pub(super) fn apply_multicursor_typing(&mut self, text: &str) -> bool {
        self.each_cursor(Kind::Typing, |_, range| Some((range, text.to_owned())))
    }

    pub(super) fn apply_multicursor_delete(&mut self, forward: bool) -> bool {
        self.each_cursor(Kind::Deleting, |doc, range| {
            let range = if !range.is_empty() {
                range
            } else if forward {
                range.start..range.start + doc.next_grapheme(range.start)
            } else {
                range.start - doc.previous_grapheme(range.start)..range.start
            };
            (!range.is_empty()).then_some((range, String::new()))
        })
    }

    fn each_cursor(&mut self, kind: Kind, edit: impl Fn(&Document, Range<usize>) -> Option<(Range<usize>, String)>) -> bool {
        if self.extra_cursors.is_empty() {
            return false;
        }
        let mut ranges: Vec<Range<usize>> = self.extra_cursors.iter().chain([&(self.anchor, self.focus)]).map(|&(a, f)| a.min(f)..a.max(f)).collect();
        ranges.sort_unstable_by_key(|range| (range.start, range.end));
        ranges.dedup_by(|next, previous| next.start < previous.end || next == previous);
        let edits = ranges.into_iter().filter_map(|range| edit(&self.doc, range)).collect();
        self.dispatch(edits, None, kind);
        for (anchor, focus) in std::iter::once((&mut self.anchor, &mut self.focus)).chain(self.extra_cursors.iter_mut().map(|(a, f)| (a, f))) {
            let end = (*anchor).max(*focus);
            (*anchor, *focus) = (end, end);
        }
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
        assert_eq!(editor.doc.whole(), "abXYcdeXYfghXYij");
        assert_eq!(cursors(&editor), (4, vec![9, 14]));
        editor.apply_multicursor_delete(false);
        assert_eq!(editor.doc.whole(), "abXcdeXfghXij");
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
        assert_eq!(editor.doc.whole(), "ab  c");
    }
}
