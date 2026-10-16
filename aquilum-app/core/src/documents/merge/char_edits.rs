use super::line_diff::LineHunk;
use crate::text::utf16::{utf16_len, utf16_seek, utf16_slice};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

pub fn apply_edits(text: &str, edits: &[TextEdit]) -> String {
    let mut ordered: Vec<&TextEdit> = edits.iter().collect();
    ordered.sort_by_key(|edit| edit.from);
    let mut result = String::with_capacity(text.len());
    let mut cursor = (0, 0);
    for edit in ordered {
        let from = utf16_seek(text, cursor, edit.from);
        result.push_str(&text[cursor.0..from.0]);
        result.push_str(&edit.insert);
        cursor = utf16_seek(text, from, edit.to);
    }
    result.push_str(&text[cursor.0..]);
    result
}

fn line_offsets(lines: &[&str]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    let mut position = 0;
    for line in lines {
        offsets.push(position);
        position += utf16_len(line) + 1;
    }
    offsets.push(position);
    offsets
}

fn terminated_edits(offsets: &[usize], hunks: &[LineHunk]) -> Vec<TextEdit> {
    hunks
        .iter()
        .map(|hunk| TextEdit {
            from: offsets[hunk.from],
            to: offsets[hunk.to],
            insert: hunk.lines.iter().map(|line| format!("{line}\n")).collect(),
        })
        .collect()
}

fn drop_terminator(insert: &str) -> &str {
    &insert[..insert.len() - 1]
}

fn without_terminator(edit: TextEdit, text_length: usize) -> TextEdit {
    if edit.to <= text_length {
        return edit;
    }
    if edit.from > text_length {
        let insert = if edit.insert.is_empty() {
            String::new()
        } else {
            format!("\n{}", drop_terminator(&edit.insert))
        };
        return TextEdit { from: text_length, to: text_length, insert };
    }
    if !edit.insert.is_empty() {
        let insert = drop_terminator(&edit.insert).to_owned();
        return TextEdit { from: edit.from, to: text_length, insert };
    }
    TextEdit { from: edit.from.saturating_sub(1), to: text_length, insert: String::new() }
}

fn with_tail_joined(mut edits: Vec<TextEdit>, text: &str) -> Vec<TextEdit> {
    if edits.len() < 2 {
        return edits;
    }
    let tail = edits.pop().expect("at least two edits");
    let ahead = edits.pop().expect("at least two edits");
    if tail.from > ahead.to {
        edits.push(ahead);
        edits.push(tail);
        return edits;
    }
    let bridged = if ahead.to > tail.from {
        let kept = utf16_len(&ahead.insert).saturating_sub(ahead.to - tail.from);
        utf16_slice(&ahead.insert, 0, kept).to_owned()
    } else {
        ahead.insert.clone() + utf16_slice(text, ahead.to, tail.from)
    };
    edits.push(TextEdit {
        from: ahead.from,
        to: ahead.to.max(tail.to),
        insert: bridged + &tail.insert,
    });
    edits
}

pub fn to_char_edits(lines: &[&str], hunks: &[LineHunk]) -> Vec<TextEdit> {
    if hunks.is_empty() {
        return Vec::new();
    }
    let text = lines.join("\n");
    let mut edits = terminated_edits(&line_offsets(lines), hunks);
    let last = edits.pop().expect("hunks are not empty");
    edits.push(without_terminator(last, utf16_len(&text)));
    with_tail_joined(edits, &text)
}
