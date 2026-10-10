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
    number: Option<(u64, char)>,
    content: usize,
}

impl Item<'_> {
    fn parse(line: &str) -> Option<Item<'_>> {
        let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let rest = &line[indent.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let (number, marker) = if rest.starts_with('-') {
            (None, 1)
        } else {
            let suffix = rest[digits..].chars().next().filter(|c| digits > 0 && matches!(c, '.' | ')'))?;
            (Some((rest[..digits].parse().ok()?, suffix)), digits + 1)
        };
        rest[marker..].starts_with(' ').then_some(Item { indent, number, content: indent.len() + marker + 1 })
    }

    fn marker(&self) -> String {
        self.number.map_or_else(|| "- ".to_owned(), |(n, suffix)| format!("{n}{suffix} "))
    }

    fn next(&self) -> String {
        self.number.map_or_else(|| "- ".to_owned(), |(n, suffix)| format!("{}{suffix} ", n + 1))
    }

    fn hanging(&self) -> String {
        format!("{}{}", self.indent, " ".repeat(self.content - self.indent.len()))
    }
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
