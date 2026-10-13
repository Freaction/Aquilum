use super::*;
use aquilum_core::markdown::line;

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn key_char(c: char) -> bool {
    word_char(c) || c == ' '
}

impl TextEditor {
    fn in_frontmatter(&self) -> bool {
        crate::frontmatter::end(self.doc.text()).is_some_and(|end| self.focus < end)
    }

    pub(super) fn property_suggest(&self) -> Option<Suggest> {
        if self.single_line || !self.selection().is_empty() || !self.in_frontmatter() {
            return None;
        }
        let (text, caret) = (self.doc.text(), self.focus);
        let Range { start: line_start, end: line_end } = line::bounds(text, caret);
        let before = &text[line_start..caret];
        let (from, to, items, source) = match before.split_once(':') {
            Some((key, value)) if crate::frontmatter::is_key(key, crate::frontmatter::PAGE_COVER) => {
                let from = line_start + key.len() + 1 + (value.len() - value.trim_start().len());
                (from, line_end, crate::cover::suggest_patterns(&text[from..caret]), Source::Value)
            }
            Some(_) => return None,
            None => {
                let key = before.trim_start();
                if key.is_empty() || !key.starts_with(char::is_alphanumeric) || !key.chars().all(key_char) || !text[caret..line_end].trim().is_empty() {
                    return None;
                }
                let from = line_start + before.len() - key.len();
                (from, caret, crate::frontmatter::suggest_keys(key), Source::Key)
            }
        };
        Some(Suggest { from, to, query: text[from..caret].to_owned(), items, selected: 0, source })
    }

    pub(super) fn suggest_context(&self) -> Option<(usize, usize, String, Source)> {
        if self.single_line || self.doc.markdown.is_none() || !self.selection().is_empty() {
            return None;
        }
        let caret = self.focus;
        let paragraph = self.doc.paragraph(self.doc.at_offset(caret));
        if paragraph.block.is_code() {
            return None;
        }
        let text = self.doc.text();
        let Range { start: line_start, end: line_end } = line::bounds(text, caret);
        let before = &text[line_start..caret];
        if before.matches('`').count() % 2 == 1 {
            return None;
        }
        if let Some(open) = before.rfind(OPEN) {
            let query = &before[open + OPEN.len()..];
            if !query.contains(CLOSE) {
                if query.contains(['|', '#']) {
                    return None;
                }
                let after = &text[caret..line_end];
                let stop = after.char_indices().find(|&(at, c)| matches!(c, '|' | '#') || after[at..].starts_with(CLOSE)).map(|(at, _)| at);
                return Some((line_start + open + OPEN.len(), caret + stop.unwrap_or(0), query.to_owned(), Source::Link { wrap: false, close: stop.is_none() }));
            }
        }
        if self.in_frontmatter() {
            return None;
        }
        let start = before.char_indices().rev().take_while(|&(_, c)| word_char(c)).last().map(|(i, _)| i)?;
        if before[..start].ends_with('#') {
            return None;
        }
        Some((line_start + start, caret, before[start..].to_owned(), Source::Link { wrap: true, close: true }))
    }
}
