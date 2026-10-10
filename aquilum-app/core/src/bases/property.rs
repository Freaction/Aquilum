use super::{BaseError, BaseRow};
use serde_json::{Number, Value};
use std::cmp::Ordering;
use std::path::Path;

pub(super) enum Property {
    Note(String),
    File(String),
}

impl Property {
    pub(super) fn parse(raw: &str) -> Result<Self, BaseError> {
        let raw = raw.trim();
        if let Some(key) = raw.strip_prefix("file.") {
            return match key {
                "path" | "name" | "ext" | "folder" | "tags" => Ok(Self::File(key.to_owned())),
                _ => Err(BaseError::UnsupportedProperty(raw.to_owned())),
            };
        }
        if let Some(inner) = raw
            .strip_prefix("note[")
            .and_then(|raw| raw.strip_suffix(']'))
        {
            return string_literal(inner)
                .filter(|key| !key.is_empty())
                .map(Self::Note)
                .ok_or_else(|| BaseError::UnsupportedProperty(raw.to_owned()));
        }
        let key = raw.strip_prefix("note.").unwrap_or(raw);
        if !key.is_empty()
            && !key.starts_with(|c: char| c.is_ascii_digit())
            && key.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            return Ok(Self::Note(key.to_owned()));
        }
        Err(BaseError::UnsupportedProperty(raw.to_owned()))
    }

    pub(super) fn value(&self, row: &BaseRow) -> Value {
        match self {
            Self::Note(key) => row.fields.get(key).cloned().unwrap_or(Value::Null),
            Self::File(key) => {
                let path = Path::new(&row.path);
                let text = match key.as_str() {
                    "path" => &row.path,
                    "name" => path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                    "ext" => path.extension().and_then(|s| s.to_str()).unwrap_or(""),
                    "folder" => path.parent().and_then(|s| s.to_str()).unwrap_or(""),
                    "tags" => {
                        return Value::Array(
                            row.file_tags.iter().cloned().map(Value::String).collect(),
                        )
                    }
                    _ => unreachable!(),
                };
                Value::String(text.to_owned())
            }
        }
    }
}

pub(super) fn string_literal(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.starts_with('"') {
        return serde_json::from_str(raw).ok();
    }
    let inner = raw.strip_prefix('\'')?.strip_suffix('\'')?;
    let mut json = String::from("\"");
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        match character {
            '\'' => return None,
            '"' => json.push_str("\\\""),
            '\\' => match chars.next()? {
                '\'' => json.push('\''),
                escaped => {
                    json.push('\\');
                    json.push(escaped);
                }
            },
            character => json.push(character),
        }
    }
    json.push('"');
    serde_json::from_str(&json).ok()
}

pub(super) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            compare_numbers(left, right) == Ordering::Equal
        }
        _ => left == right,
    }
}

pub(super) fn compare_numbers(left: &Number, right: &Number) -> Ordering {
    match (integer(left), integer(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(left), None) => compare_integer_float(left, right.as_f64().unwrap()),
        (None, Some(right)) => compare_integer_float(right, left.as_f64().unwrap()).reverse(),
        (None, None) => left.as_f64().partial_cmp(&right.as_f64()).unwrap(),
    }
}

fn integer(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

fn compare_integer_float(integer: i128, float: f64) -> Ordering {
    integer
        .cmp(&(float.trunc() as i128))
        .then_with(|| 0.0_f64.partial_cmp(&float.fract()).unwrap())
}
