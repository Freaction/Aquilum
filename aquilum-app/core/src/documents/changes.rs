use super::merge::TextEdit;
use crate::text::utf16::utf16_len;
use serde_json::{json, Value};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct ChangeError(String);

impl fmt::Display for ChangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn invalid(message: &str) -> ChangeError {
    ChangeError(message.to_owned())
}

fn section_len(value: &Value) -> Result<usize, ChangeError> {
    value.as_u64().map(|length| length as usize).ok_or_else(|| invalid("section length is not a number"))
}

pub fn edits_from_json(changes: &Value, doc_length: usize) -> Result<Vec<TextEdit>, ChangeError> {
    let sections = changes.as_array().ok_or_else(|| invalid("change set is not an array"))?;
    let mut position = 0;
    let mut edits = Vec::new();
    for section in sections {
        if let Some(parts) = section.as_array() {
            let (removed, lines) = parts.split_first().ok_or_else(|| invalid("empty replacement"))?;
            let removed = section_len(removed)?;
            let insert = lines
                .iter()
                .map(|line| line.as_str().ok_or_else(|| invalid("inserted line is not a string")))
                .collect::<Result<Vec<_>, _>>()?
                .join("\n");
            edits.push(TextEdit { from: position, to: position + removed, insert });
            position += removed;
        } else {
            position += section_len(section)?;
        }
    }
    if position != doc_length {
        return Err(invalid("change set does not cover the document"));
    }
    Ok(edits)
}

pub fn edits_to_json(edits: &[TextEdit], doc_length: usize) -> Value {
    let mut sections = Vec::with_capacity(edits.len() * 2 + 1);
    let mut position = 0;
    for edit in edits {
        if edit.from > position {
            sections.push(json!(edit.from - position));
        }
        let mut replacement = vec![json!(edit.to - edit.from)];
        if !edit.insert.is_empty() {
            replacement.extend(edit.insert.split('\n').map(|line| json!(line)));
        }
        sections.push(Value::Array(replacement));
        position = edit.to;
    }
    if doc_length > position {
        sections.push(json!(doc_length - position));
    }
    Value::Array(sections)
}

pub fn length_after(edits: &[TextEdit], doc_length: usize) -> usize {
    edits.iter().fold(doc_length, |length, edit| length - (edit.to - edit.from) + utf16_len(&edit.insert))
}

#[cfg(test)]
#[path = "changes_tests.rs"]
mod tests;
