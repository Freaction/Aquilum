//! Ctrl+B, Ctrl+I, Ctrl+Shift+S: разметка выделения в пределах строки. Маркер ищется как группа
//! одинаковых символов у края текста: курсив — нечётная группа `*` (`*` или `***`), жирный и
//! зачёркнутый — группа не короче двух.

use super::*;

pub(super) const BOLD: &str = "**";
pub(super) const ITALIC: &str = "*";
pub(super) const STRIKE: &str = "~~";

fn run_before(text: &[u8], at: usize, c: u8, stop: usize) -> usize {
    text[stop..at].iter().rev().take_while(|&&b| b == c).count()
}

fn run_after(text: &[u8], at: usize, c: u8, stop: usize) -> usize {
    text[at..stop].iter().take_while(|&&b| b == c).count()
}

fn side(mark: &str, run: usize) -> bool {
    if mark.len() == 1 { run % 2 == 1 } else { run >= 2 }
}

fn marked(mark: &str, before: usize, after: usize) -> bool {
    side(mark, before) && side(mark, after)
}

impl TextEditor {
    pub(super) fn toggle_mark(&mut self, mark: &str) -> bool {
        let range = self.selection();
        let text = self.doc.text();
        let bytes = text.as_bytes();
        let (c, n) = (mark.as_bytes()[0], mark.len());
        let line_start = text[..range.start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[range.end..].find('\n').map_or(text.len(), |i| range.end + i);
        let (before, after) = (run_before(bytes, range.start, c, line_start), run_after(bytes, range.end, c, line_end));
        let (inner_before, inner_after) = (run_after(bytes, range.start, c, range.end), run_before(bytes, range.end, c, range.start));
        let unwrap = |from: usize, to: usize| (from - n..to + n, text[from..to].to_owned(), (range.start.max(from) - n, range.end.min(to) - n));
        let (outer, inner, selection) = if marked(mark, before, after) {
            unwrap(range.start, range.end)
        } else if inner_before + inner_after < range.len() && marked(mark, inner_before, inner_after) {
            let (from, to) = (range.start + inner_before, range.end - inner_after);
            let inner = format!("{}{}{}", &text[range.start..from - n], &text[from..to], &text[to + n..range.end]);
            (range.clone(), inner, (range.start, range.end - 2 * n))
        } else if let (Some(from), Some(to)) = (
            (line_start..=range.start).rev().find(|&p| p > line_start && bytes[p - 1] == c && bytes.get(p) != Some(&c) && side(mark, run_before(bytes, p, c, line_start))),
            (range.end..line_end).find(|&q| bytes[q] == c && (q == 0 || bytes[q - 1] != c) && side(mark, run_after(bytes, q, c, line_end))),
        ) {
            unwrap(from, to)
        } else if range.is_empty() {
            return false;
        } else {
            (range.clone(), format!("{mark}{}{mark}", &text[range.clone()]), (range.start + n, range.end + n))
        };
        let forward = self.anchor <= self.focus;
        self.replace(outer, &inner, Kind::Other);
        (self.anchor, self.focus) = if forward { selection } else { (selection.1, selection.0) };
        true
    }
}
