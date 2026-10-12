use super::*;
use aquilum_core::markdown::indent::TAB;
use aquilum_core::markdown::line;

fn outdent_width(line: &str) -> usize {
    if line.starts_with('\t') {
        return 1;
    }
    line.bytes().take(TAB).take_while(|b| *b == b' ').count()
}

impl TextEditor {
    pub(in super::super) fn indent(&mut self, more: bool) -> bool {
        let text = self.doc.text();
        let range = self.selection();
        let last = if range.end > range.start && text[..range.end].ends_with('\n') { range.end - 1 } else { range.end };
        let (start, end) = (line::bounds(text, range.start).start, line::bounds(text, last).end);
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
