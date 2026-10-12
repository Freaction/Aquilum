use super::*;
use aquilum_core::markdown::list as grammar;

pub(super) struct Item<'a> {
    pub(super) indent: &'a str,
    pub(super) columns: usize,
    bullet: char,
    pub(super) number: Option<u64>,
    task: bool,
    pub(super) marker: Range<usize>,
    pub(super) content: usize,
}

impl Item<'_> {
    pub(super) fn parse(line: &str) -> Option<Item<'_>> {
        let found = grammar::item(line)?;
        Some(Item {
            indent: &line[..found.indent.bytes],
            columns: found.indent.columns,
            bullet: found.bullet,
            number: found.number,
            task: found.task.is_some(),
            marker: found.marker,
            content: found.content,
        })
    }

    pub(super) fn content_column(&self) -> usize {
        self.columns + self.marker.len()
    }

    fn prefix(&self, number: Option<u64>) -> String {
        let task = if self.task { "[ ] " } else { "" };
        match number {
            Some(n) => format!("{n}{} {task}", self.bullet),
            None => format!("{} {task}", self.bullet),
        }
    }

    pub(super) fn marker(&self) -> String {
        self.prefix(self.number)
    }

    pub(super) fn next(&self) -> String {
        self.prefix(self.number.map(|n| n + 1))
    }

    pub(super) fn hanging(&self) -> String {
        format!("{}{}", self.indent, " ".repeat(self.marker.len()))
    }
}
