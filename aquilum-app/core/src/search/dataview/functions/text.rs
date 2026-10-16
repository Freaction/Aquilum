use super::first;
use super::super::value::Value;
use crate::text::matcher::{Matcher, IGNORE_CASE};

pub fn length(value: &Value) -> usize {
    match value {
        Value::Null => 0,
        Value::List(items) => items.len(),
        Value::Text(text) => text.chars().count(),
        other => other.text().chars().count(),
    }
}

pub fn contains(arguments: &[Value]) -> bool {
    let haystack = first(arguments);
    let needle = arguments.get(1).unwrap_or(&Value::Null);
    let matcher = Matcher::new(&needle.text(), IGNORE_CASE);
    let inside = |text: String| matcher.first(&text).is_some();
    match haystack {
        Value::List(items) => items.iter().any(|item| item.equals(needle) || matcher.is_empty() || inside(item.text())),
        Value::Null => false,
        other => inside(other.text()),
    }
}

pub fn contains_word(arguments: &[Value]) -> bool {
    let wanted = arguments.get(1).map(Value::text).unwrap_or_default().to_lowercase();
    if wanted.is_empty() {
        return false;
    }
    let inside = |text: String| {
        text.to_lowercase()
            .split(|symbol: char| !symbol.is_alphanumeric() && symbol != '_')
            .any(|word| word == wanted)
    };
    match first(arguments) {
        Value::List(items) => items.iter().any(|item| inside(item.text())),
        other => inside(other.text()),
    }
}

pub fn text_of(value: &Value, apply: fn(&str) -> String) -> Value {
    match value {
        Value::Null => Value::Null,
        other => Value::Text(apply(&other.text())),
    }
}

pub fn join(arguments: &[Value]) -> Value {
    let separator = arguments
        .get(1)
        .map(Value::text)
        .unwrap_or_else(|| ", ".to_owned());
    match first(arguments) {
        Value::List(items) => Value::Text(
            items
                .iter()
                .map(Value::text)
                .collect::<Vec<_>>()
                .join(&separator),
        ),
        Value::Null => Value::Null,
        other => Value::Text(other.text()),
    }
}

pub fn split(arguments: &[Value]) -> Value {
    let text = first(arguments).text();
    let separator = arguments.get(1).map(Value::text).unwrap_or_default();
    if separator.is_empty() {
        return Value::List(text.chars().map(|item| Value::Text(item.to_string())).collect());
    }
    Value::List(text.split(&separator).map(|part| Value::Text(part.to_owned())).collect())
}

pub fn replace(arguments: &[Value]) -> Value {
    let text = first(arguments).text();
    let what = arguments.get(1).map(Value::text).unwrap_or_default();
    let with = arguments.get(2).map(Value::text).unwrap_or_default();
    if what.is_empty() {
        return Value::Text(text);
    }
    Value::Text(text.replace(&what, &with))
}

pub fn substring(arguments: &[Value]) -> Value {
    let text = first(arguments).text();
    let symbols: Vec<char> = text.chars().collect();
    let from = arguments.get(1).and_then(Value::as_number).unwrap_or(0.0).max(0.0) as usize;
    let to = arguments
        .get(2)
        .and_then(Value::as_number)
        .map(|value| value.max(0.0) as usize)
        .unwrap_or(symbols.len())
        .min(symbols.len());
    if from >= to {
        return Value::Text(String::new());
    }
    Value::Text(symbols[from..to].iter().collect())
}

pub fn ends(arguments: &[Value], at_start: bool) -> Value {
    let text = first(arguments).text().to_lowercase();
    let part = arguments.get(1).map(Value::text).unwrap_or_default().to_lowercase();
    Value::Bool(if at_start { text.starts_with(&part) } else { text.ends_with(&part) })
}

pub fn pad(arguments: &[Value], at_start: bool) -> Value {
    let text = first(arguments).text();
    let width = arguments.get(1).and_then(Value::as_number).unwrap_or(0.0).max(0.0) as usize;
    let filler = arguments
        .get(2)
        .map(Value::text)
        .and_then(|value| value.chars().next())
        .unwrap_or(' ');
    let length = text.chars().count();
    if length >= width {
        return Value::Text(text);
    }
    let padding: String = std::iter::repeat_n(filler, width - length).collect();
    Value::Text(if at_start { padding + &text } else { text + &padding })
}

pub fn truncate(arguments: &[Value]) -> Value {
    let text = first(arguments).text();
    let width = arguments.get(1).and_then(Value::as_number).unwrap_or(0.0).max(0.0) as usize;
    let tail = arguments.get(2).map(Value::text).unwrap_or_else(|| "…".to_owned());
    let symbols: Vec<char> = text.chars().collect();
    if symbols.len() <= width {
        return Value::Text(text);
    }
    let keep = width.saturating_sub(tail.chars().count());
    Value::Text(symbols[..keep].iter().collect::<String>() + &tail)
}
