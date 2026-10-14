use super::*;
use aquilum_core::markdown::list as grammar;

pub(super) struct Item<'a> {
    pub(super) indent: &'a str,
    found: grammar::Item,
}

impl Item<'_> {
    pub(super) fn parse(line: &str) -> Option<Item<'_>> {
        let found = grammar::item(line)?;
        Some(Item { indent: &line[..found.indent.bytes], found })
    }

    pub(super) fn content(&self) -> usize {
        self.found.content
    }

    pub(super) fn content_column(&self) -> usize {
        self.found.content_column()
    }

    pub(super) fn number(&self) -> Option<(Range<usize>, u64)> {
        let marker = &self.found.marker;
        self.found.number.map(|n| (marker.start..marker.end - 2, n))
    }

    pub(super) fn next(&self) -> String {
        let task = if self.found.task.is_some() { "[ ] " } else { "" };
        match self.found.number {
            Some(n) => format!("{}{} {task}", n + 1, self.found.bullet),
            None => format!("{} {task}", self.found.bullet),
        }
    }

    pub(super) fn hanging(&self) -> String {
        format!("{}{}", self.indent, " ".repeat(self.found.marker.len()))
    }
}
