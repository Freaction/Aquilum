use super::*;
use aquilum_core::markdown::indent::TAB;
use aquilum_core::markdown::line;

fn outdent_width(line: &str) -> usize {
    if line.starts_with('\t') {
        return 1;
    }
    line.bytes().take(TAB).take_while(|b| *b == b' ').count()
}

impl TextEditor {
    pub(in super::super) fn indent(&mut self, more: bool) -> bool {
        let text = self.doc.text();
        let range = self.selection();
        let last = if range.end > range.start && text[..range.end].ends_with('\n') { range.end - 1 } else { range.end };
        let (start, end) = (line::bounds(text, range.start).start, line::bounds(text, last).end);
        let several = text[start..end].contains('\n');
        let deepest = |at: usize| matches!(&self.doc.paragraph(self.doc.at_offset(at)).block, Block::Item(item) if item.deepest());
        let mut edits = Vec::new();
        let mut at = start;
        for line in text[start..end].split('\n') {
            if !more {
                edits.push((at..at + outdent_width(line), String::new()));
            } else if !(several && line.trim().is_empty() || deepest(at)) {
                edits.push((at..at, "\t".to_owned()));
            }
            at += line.len() + 1;
        }
        edits.retain(|(range, text)| !range.is_empty() || !text.is_empty());
        if edits.is_empty() {
            return false;
        }
        self.dispatch(edits, None, Kind::Other);
        true
    }
}
