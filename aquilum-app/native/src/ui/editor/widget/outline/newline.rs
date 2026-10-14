use super::item::Item;
use super::*;
use aquilum_core::markdown::indent::Indent;

impl TextEditor {
    fn continued(&self, i: usize) -> Option<Item<'_>> {
        let Block::Continuation { level, .. } = self.doc.paragraph(i).block else { return None };
        let owner = (0..i).rev().map(|j| self.doc.paragraph(j)).find(|p| matches!(&p.block, Block::Item(item) if item.level() == level))?;
        let item = Item::parse(&self.doc.text()[owner.range()])?;
        let line = &self.doc.text()[self.doc.paragraph(i).range()];
        (Indent::of(line).columns == item.content_column()).then_some(item)
    }

    pub(in super::super) fn newline(&mut self, hard: bool) -> bool {
        let range = self.selection();
        if !range.is_empty() {
            return false;
        }
        let at = range.start;
        let i = self.doc.at_offset(at);
        let p = self.doc.paragraph(i);
        if p.block.is_code() {
            return false;
        }
        let Range { start, end } = p.range();
        let line = &self.doc.text()[start..end];
        let item = Item::parse(line);
        if item.as_ref().is_some_and(|item| at < start + item.content()) {
            return false;
        }
        let (range, insert) = if hard {
            let Some(open) = item.or_else(|| self.continued(i)) else { return false };
            (at..at, format!("\n{}", open.hanging()))
        } else if let Some(item) = item {
            match (line[item.content()..].trim().is_empty(), item.indent.is_empty()) {
                (true, false) => return self.indent(false),
                (true, true) => (start..end, String::new()),
                (false, _) => (at..at, format!("\n{}{}", item.indent, item.next())),
            }
        } else if let Some(open) = self.continued(i) {
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
