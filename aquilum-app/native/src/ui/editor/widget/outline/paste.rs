use super::item::Item;
use super::*;
use aquilum_core::markdown::list as grammar;

fn unmarked(line: &str) -> &str {
    let bare = line.trim_start();
    let content = bare.strip_prefix('•').map(str::trim_start).or_else(|| grammar::item(bare).map(|item| &bare[item.content..]));
    content.unwrap_or(line).trim_end()
}

impl TextEditor {
    pub(in super::super) fn pasted_items(&self, text: &str) -> Option<String> {
        let range = self.selection();
        let p = self.doc.paragraph(self.doc.at_offset(range.start));
        if self.doc.markdown.is_none() || !text.contains('\n') || range.end > p.end() {
            return None;
        }
        let item = Item::parse(self.doc.slice(p.range()))?;
        if range.start < p.start + item.content() {
            return None;
        }
        let marker = format!("\n{}{}", item.indent, item.next());
        let mut lines = text.lines().map(unmarked);
        let first = lines.next()?.to_owned();
        Some(lines.map(str::trim_start).filter(|line| !line.is_empty()).fold(first, |out, line| out + &marker + line))
    }
}
