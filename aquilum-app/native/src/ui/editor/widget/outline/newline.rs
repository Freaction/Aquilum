use super::item::Item;
use super::*;
use aquilum_core::markdown::indent::{Indent, TAB};
use aquilum_core::markdown::line;

fn reduce(indent: &str) -> Option<&str> {
    if let Some(rest) = indent.strip_suffix('\t') {
        return Some(rest);
    }
    let spaces = indent.len() - indent.trim_end_matches(' ').len();
    (spaces > 0).then(|| &indent[..indent.len() - spaces.min(TAB)])
}

fn open_item<'a>(before: &'a str, line: &str) -> Option<Item<'a>> {
    let columns = Indent::of(line).columns;
    if columns == 0 {
        return None;
    }
    for text in before.rsplit('\n') {
        if text.trim().is_empty() {
            return None;
        }
        if let Some(item) = Item::parse(text) {
            return (item.content_column() == columns).then_some(item);
        }
        if Indent::of(text).columns != columns {
            return None;
        }
    }
    None
}

impl TextEditor {
    pub(in super::super) fn newline(&mut self, hard: bool) -> bool {
        let range = self.selection();
        if !range.is_empty() {
            return false;
        }
        let text = self.doc.text();
        let at = range.start;
        if self.doc.paragraph(self.doc.at_offset(at)).block.is_code() {
            return false;
        }
        let Range { start, end } = line::bounds(text, at);
        let line = &text[start..end];
        let before = &text[..start.saturating_sub(1)];
        let item = Item::parse(line);
        if item.as_ref().is_some_and(|item| at < start + item.content) {
            return false;
        }
        let (range, insert) = if hard {
            let Some(open) = item.or_else(|| open_item(before, line)) else { return false };
            (at..at, format!("\n{}", open.hanging()))
        } else if let Some(item) = item {
            if line[item.content..].trim().is_empty() {
                let prefix = reduce(item.indent).map(|indent| format!("{indent}{}", item.marker())).unwrap_or_default();
                (start..end, prefix)
            } else {
                (at..at, format!("\n{}{}", item.indent, item.next()))
            }
        } else if let Some(open) = open_item(before, line) {
            (at..at, format!("\n{}{}", open.indent, open.next()))
        } else if line.trim_start().starts_with('>') {
            let quote = &line[..line.len() - line.trim_start_matches([' ', '\t', '>']).len()];
            if line.trim() == ">" {
                (start..end, "\n".to_owned())
            } else {
                let quote = if quote.ends_with(' ') { quote.to_owned() } else { format!("{quote} ") };
                (at..at, format!("\n{quote}"))
            }
        } else {
            return false;
        };
        self.replace(range, &insert, Kind::Other);
        true
    }
}
