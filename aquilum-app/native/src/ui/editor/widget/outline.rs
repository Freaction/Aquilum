use super::*;

const TAB_SIZE: usize = 4;

fn outdent_width(line: &str) -> usize {
    if line.starts_with('\t') {
        return 1;
    }
    line.bytes().take(TAB_SIZE).take_while(|b| *b == b' ').count()
}

impl TextEditor {
    pub(super) fn indent(&mut self, more: bool) -> bool {
        let text = self.doc.text();
        let range = self.selection();
        let start = text[..range.start].rfind('\n').map_or(0, |i| i + 1);
        let last = if range.end > range.start && text[..range.end].ends_with('\n') { range.end - 1 } else { range.end };
        let end = text[last..].find('\n').map_or(text.len(), |i| last + i);
        let lines: Vec<&str> = text[start..end].split('\n').collect();
        let several = lines.len() > 1;
        let mut out = String::with_capacity(end - start + lines.len());
        let mut shifts = Vec::new();
        let mut at = start;
        for (n, line) in lines.iter().enumerate() {
            if n > 0 {
                out.push('\n');
            }
            let delta = match more {
                true if several && line.trim().is_empty() => 0,
                true => {
                    out.push('\t');
                    1
                }
                false => -(outdent_width(line) as isize),
            };
            out.push_str(&line[delta.min(0).unsigned_abs()..]);
            if delta != 0 {
                shifts.push((at, delta));
            }
            at += line.len() + 1;
        }
        if shifts.is_empty() {
            return false;
        }
        let map = |offset: usize| {
            let mut moved = offset as isize;
            for &(line, delta) in &shifts {
                if offset < line {
                    break;
                }
                moved += if delta > 0 { delta } else { -((offset - line).min(delta.unsigned_abs()) as isize) };
            }
            moved.max(0) as usize
        };
        let (anchor, focus) = (map(self.anchor), map(self.focus));
        self.replace(start..end, &out, Kind::Other);
        let len = self.doc.len();
        (self.anchor, self.focus) = (anchor.min(len), focus.min(len));
        true
    }
}

struct Item<'a> {
    indent: &'a str,
    bullet: char,
    number: Option<u64>,
    task: bool,
    marker: Range<usize>,
    content: usize,
}

impl Item<'_> {
    fn parse(line: &str) -> Option<Item<'_>> {
        let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let rest = &line[indent.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let (bullet, number, width) = match rest.chars().next()? {
            c @ ('-' | '*' | '+') => (c, None, 1),
            _ => {
                let suffix = rest[digits..].chars().next().filter(|c| digits > 0 && matches!(c, '.' | ')'))?;
                (suffix, Some(rest[..digits].parse().ok()?), digits + 1)
            }
        };
        if !rest[width..].starts_with(' ') {
            return None;
        }
        let marker = indent.len()..indent.len() + width + 1;
        let after = line[marker.end..].as_bytes();
        let task = after.len() >= 3 && after[0] == b'[' && after[2] == b']' && matches!(after.get(3), None | Some(b' '));
        let content = if task { (marker.end + 4).min(line.len()) } else { marker.end };
        Some(Item { indent, bullet, number, task, marker, content })
    }

    fn prefix(&self, number: Option<u64>) -> String {
        let task = if self.task { "[ ] " } else { "" };
        match number {
            Some(n) => format!("{n}{} {task}", self.bullet),
            None => format!("{} {task}", self.bullet),
        }
    }

    fn marker(&self) -> String {
        self.prefix(self.number)
    }

    fn next(&self) -> String {
        self.prefix(self.number.map(|n| n + 1))
    }

    fn hanging(&self) -> String {
        format!("{}{}", self.indent, " ".repeat(self.marker.len()))
    }
}

fn level(indent: &str) -> Option<usize> {
    let columns = indent.chars().fold(0, |columns, c| if c == '\t' { columns + TAB_SIZE - columns % TAB_SIZE } else { columns + 1 });
    (columns % TAB_SIZE == 0).then_some(columns / TAB_SIZE)
}

fn listed(line: &str) -> bool {
    line.trim().is_empty() || Item::parse(line).and_then(|item| level(item.indent)).is_some()
}

pub(super) fn renumber(text: &str, around: Range<usize>) -> Option<(Range<usize>, String)> {
    let line_start = |at: usize| text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = |at: usize| text[at..].find('\n').map_or(text.len(), |i| at + i);
    let mut from = line_start(around.start);
    while from > 0 && listed(&text[line_start(from - 1)..from - 1]) {
        from = line_start(from - 1);
    }
    let mut to = line_end(around.end);
    while to < text.len() && listed(&text[to + 1..line_end(to + 1)]) {
        to = line_end(to + 1);
    }
    let mut counters: Vec<Option<u64>> = Vec::new();
    let mut changes: Vec<(Range<usize>, String)> = Vec::new();
    let mut start = from;
    for line in text[from..to].split('\n') {
        let at = start;
        start += line.len() + 1;
        let item = Item::parse(line);
        let Some((item, level)) = item.and_then(|item| level(item.indent).map(|level| (item, level))) else {
            if !line.trim().is_empty() {
                counters.clear();
            }
            continue;
        };
        counters.resize(level + 1, None);
        let Some(number) = item.number else {
            counters[level] = None;
            continue;
        };
        let expected = counters[level].map_or(1, |n| n + 1);
        counters[level] = Some(expected);
        if number != expected {
            changes.push((at + item.marker.start..at + item.marker.end - 2, expected.to_string()));
        }
    }
    let (first, last) = (changes.first()?.0.start, changes.last()?.0.end);
    let mut out = String::new();
    let mut cursor = first;
    for (range, number) in &changes {
        out.push_str(&text[cursor..range.start]);
        out.push_str(number);
        cursor = range.end;
    }
    Some((first..last, out))
}

fn reduce(indent: &str) -> Option<&str> {
    if let Some(rest) = indent.strip_suffix('\t') {
        return Some(rest);
    }
    let spaces = indent.len() - indent.trim_end_matches(' ').len();
    (spaces > 0).then(|| &indent[..indent.len() - spaces.min(TAB_SIZE)])
}

fn open_item<'a>(before: &'a str, line: &str) -> Option<Item<'a>> {
    let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
    if indent.is_empty() {
        return None;
    }
    for text in before.rsplit('\n') {
        if text.trim().is_empty() {
            return None;
        }
        if let Some(item) = Item::parse(text) {
            return (item.hanging() == indent).then_some(item);
        }
        if !text.starts_with(indent) || text[indent.len()..].starts_with([' ', '\t']) {
            return None;
        }
    }
    None
}

impl TextEditor {
    pub(super) fn newline(&mut self, hard: bool) -> bool {
        let range = self.selection();
        if !range.is_empty() {
            return false;
        }
        let text = self.doc.text();
        let at = range.start;
        let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
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

#[cfg(test)]
mod tests {
    #[test]
    fn renumbering_touches_only_the_edited_list() {
        let text = "1. раз\n3. два\n\nабзац\n\n1. один\n5. другой";
        let at = text.find("3.").unwrap();
        let (range, numbers) = super::renumber(text, at..at).unwrap();
        let mut fixed = text.to_owned();
        fixed.replace_range(range, &numbers);
        assert_eq!(fixed, "1. раз\n2. два\n\nабзац\n\n1. один\n5. другой");
        let at = text.find("один").unwrap();
        let (range, numbers) = super::renumber(text, at..at).unwrap();
        let mut fixed = text.to_owned();
        fixed.replace_range(range, &numbers);
        assert_eq!(fixed, "1. раз\n3. два\n\nабзац\n\n1. один\n2. другой");
        assert!(super::renumber(text, 0..0).is_some());
        assert!(super::renumber("1. а\n2. б\n\nтекст", 0..0).is_none());
    }
}
