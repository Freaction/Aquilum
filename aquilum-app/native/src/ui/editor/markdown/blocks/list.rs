use std::ops::Range;

use aquilum_core::markdown::fence::INDENT_LIMIT;
use aquilum_core::markdown::list as grammar;

use super::super::brushes::LIST_CALLOUTS;

const DEPTH: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    indent: u32,
    content: u32,
    level: u16,
    boxed: u8,
    width: u8,
    pub ordered: bool,
    done: bool,
    tone: Option<u8>,
}

fn tone(after: &str) -> Option<u8> {
    let symbol = after.chars().next()?;
    let position = LIST_CALLOUTS.iter().position(|(c, _)| *c == symbol)?;
    after[symbol.len_utf8()..].starts_with(' ').then_some(position as u8)
}

impl Item {
    pub(super) fn new(found: &grammar::Item, level: usize, line: &str) -> Item {
        let tone = found.task.is_none().then(|| tone(&line[found.marker.end..])).flatten();
        let content = if tone.is_some() { found.marker.end + 2 } else { found.content };
        Item {
            indent: found.indent.bytes as u32,
            content: content as u32,
            level: level as u16,
            boxed: found.task.as_ref().map_or(0, |task| task.range.len() as u8),
            width: found.marker.len() as u8,
            ordered: found.ordered(),
            done: found.task.as_ref().is_some_and(|task| task.done),
            tone,
        }
    }

    pub fn level(&self) -> usize {
        usize::from(self.level)
    }

    pub fn indent(&self) -> usize {
        self.indent as usize
    }

    pub fn content(&self) -> usize {
        self.content as usize
    }

    pub fn marker(&self) -> Range<usize> {
        self.indent()..self.indent() + usize::from(self.width)
    }

    pub fn task(&self) -> Option<(Range<usize>, bool)> {
        let end = self.marker().end;
        (self.boxed > 0).then(|| (end..end + usize::from(self.boxed), self.done))
    }

    pub fn callout(&self) -> Option<(usize, u8)> {
        self.tone.map(|tone| (self.marker().end, tone))
    }
}

fn column(value: usize) -> u16 {
    value.min(usize::from(u16::MAX)) as u16
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct Levels {
    indent: [u16; DEPTH],
    content: [u16; DEPTH],
    len: u8,
}

impl Levels {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn last(&self) -> Option<usize> {
        self.len.checked_sub(1).map(usize::from)
    }

    pub fn accepts(&self, columns: usize) -> bool {
        !self.is_empty() || columns <= INDENT_LIMIT
    }

    pub fn enter(&mut self, indent: usize, content: usize) -> usize {
        let (indent, content) = (column(indent), column(content));
        while self.len > 0 && self.indent[usize::from(self.len) - 1] > indent {
            self.len -= 1;
        }
        let top = usize::from(self.len);
        let level = if top > 0 && self.indent[top - 1] == indent { top - 1 } else { top.min(DEPTH - 1) };
        self.indent[level] = indent;
        self.content[level] = content;
        self.len = level as u8 + 1;
        level
    }

    pub fn owner(&self, columns: usize) -> Option<usize> {
        (0..usize::from(self.len)).rev().find(|&level| usize::from(self.content[level]) <= columns)
    }

    pub fn width(&self, level: usize) -> usize {
        usize::from(self.content[level] - self.indent[level])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeper_indentation_nests_whatever_it_is_made_of() {
        let mut levels = Levels::default();
        assert_eq!(levels.enter(0, 2), 0);
        assert_eq!(levels.enter(2, 4), 1);
        assert_eq!(levels.enter(6, 8), 2);
        assert_eq!(levels.enter(6, 8), 2);
        assert_eq!(levels.enter(2, 4), 1);
        assert_eq!(levels.enter(0, 2), 0);
        assert_eq!(levels.enter(4, 6), 1);
    }

    #[test]
    fn an_indent_between_two_levels_opens_a_new_one_under_the_shallower() {
        let mut levels = Levels::default();
        levels.enter(0, 2);
        levels.enter(4, 6);
        assert_eq!(levels.enter(2, 4), 1);
        assert_eq!(levels.last(), Some(1));
    }

    #[test]
    fn a_continuation_belongs_to_the_deepest_item_it_is_indented_past() {
        let mut levels = Levels::default();
        levels.enter(0, 3);
        levels.enter(4, 6);
        assert_eq!(levels.owner(2), None);
        assert_eq!(levels.owner(3), Some(0));
        assert_eq!(levels.owner(5), Some(0));
        assert_eq!(levels.owner(6), Some(1));
        assert_eq!(levels.width(0), 3);
    }

    #[test]
    fn the_first_item_cannot_start_deeper_than_three_columns() {
        let mut levels = Levels::default();
        assert!(levels.accepts(3));
        assert!(!levels.accepts(4));
        levels.enter(0, 2);
        assert!(levels.accepts(12));
    }

    #[test]
    fn the_depth_is_capped_by_replacing_the_deepest_level() {
        let mut levels = Levels::default();
        for step in 0..DEPTH + 3 {
            levels.enter(step * 2, step * 2 + 2);
        }
        assert_eq!(levels.last(), Some(DEPTH - 1));
    }
}
