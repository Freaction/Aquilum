use std::ops::Range;

use super::indent::Indent;

const MAX_DIGITS: usize = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub range: Range<usize>,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub indent: Indent,
    pub bullet: char,
    pub number: Option<u64>,
    pub marker: Range<usize>,
    pub task: Option<Task>,
    pub content: usize,
}

impl Item {
    pub fn ordered(&self) -> bool {
        self.number.is_some()
    }

    pub fn content_column(&self) -> usize {
        self.indent.columns + self.marker.len()
    }
}

fn marker(rest: &str) -> Option<(char, Option<u64>, usize)> {
    let first = rest.chars().next()?;
    if matches!(first, '-' | '*' | '+') {
        return rest[1..].starts_with(' ').then_some((first, None, 2));
    }
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > MAX_DIGITS {
        return None;
    }
    let delimiter = rest[digits..].chars().next().filter(|c| matches!(c, '.' | ')'))?;
    if !rest[digits + 1..].starts_with(' ') {
        return None;
    }
    Some((delimiter, rest[..digits].parse().ok(), digits + 2))
}

fn task(after: &str, base: usize) -> Option<(Task, usize)> {
    let inner = after.strip_prefix('[')?;
    let mark = inner.chars().next()?;
    let closed = inner[mark.len_utf8()..].strip_prefix(']')?;
    if !closed.is_empty() && !closed.starts_with([' ', '\t']) {
        return None;
    }
    let end = base + 2 + mark.len_utf8();
    let content = end + usize::from(!closed.is_empty());
    Some((Task { range: base..end, done: !mark.is_whitespace() }, content))
}

pub fn item(line: &str) -> Option<Item> {
    let indent = Indent::of(line);
    let (bullet, number, width) = marker(&line[indent.bytes..])?;
    let marker = indent.bytes..indent.bytes + width;
    let (task, content) = match task(&line[marker.end..], marker.end) {
        Some((task, content)) => (Some(task), content),
        None => (None, marker.end),
    };
    Some(Item { indent, bullet, number, marker, task, content })
}

pub fn toggle_task(line: &str) -> Option<String> {
    let task = item(line)?.task?;
    let mark = task.range.start + 1..task.range.end - 1;
    let mut next = line.to_owned();
    next.replace_range(mark, if task.done { " " } else { "x" });
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bullet_and_ordered_marker_starts_an_item() {
        for (line, bullet, number) in [("- a", '-', None), ("* a", '*', None), ("+ a", '+', None), ("12. a", '.', Some(12)), ("3) a", ')', Some(3))] {
            let found = item(line).unwrap_or_else(|| panic!("{line}"));
            assert_eq!((found.bullet, found.number), (bullet, number), "{line}");
            assert_eq!(found.content, line.len() - 1, "{line}");
        }
        for line in ["-a", "3.a", "1234567890. a", ". a", "text", "--- ", "-", ""] {
            assert!(item(line).is_none(), "{line}");
        }
    }

    #[test]
    fn the_indent_is_measured_in_columns() {
        let found = item("\t  - вложенный").unwrap();
        assert_eq!((found.indent.bytes, found.indent.columns, found.marker.clone()), (3, 6, 3..5));
        assert_eq!(found.content_column(), 8);
    }

    #[test]
    fn a_task_box_is_any_single_mark_followed_by_a_space_or_the_end() {
        let open = item("- [ ] дело").unwrap();
        assert_eq!((open.task.clone().unwrap(), open.content), (Task { range: 2..5, done: false }, 6));
        assert!(item("- [x] дело").unwrap().task.unwrap().done);
        assert!(item("1. [✓] дело").unwrap().task.unwrap().done);
        assert_eq!(item("- [ ]").unwrap().task.unwrap().range, 2..5);
        assert_eq!(item("- [ ]").unwrap().content, 5);
        for line in ["- [x]дело", "- [] пусто", "- [ab] два", "- [ ", "- без"] {
            assert!(item(line).unwrap().task.is_none(), "{line}");
        }
    }

    #[test]
    fn toggling_flips_only_the_mark() {
        assert_eq!(toggle_task("- [ ] дело").as_deref(), Some("- [x] дело"));
        assert_eq!(toggle_task("\t1. [x] b").as_deref(), Some("\t1. [ ] b"));
        assert_eq!(toggle_task("* [✓] c").as_deref(), Some("* [ ] c"));
        assert_eq!(toggle_task("- [x]дело"), None);
        assert_eq!(toggle_task("просто текст"), None);
    }
}
