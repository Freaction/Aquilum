use super::{first, items, span_of};
use super::super::value::Value;

pub fn reverse(value: &Value) -> Value {
    match value {
        Value::List(items) => Value::List(items.iter().rev().cloned().collect()),
        other => other.clone(),
    }
}

pub fn sorted(value: &Value) -> Value {
    match value {
        Value::List(items) => {
            let mut items = items.clone();
            items.sort_by(Value::compare);
            Value::List(items)
        }
        other => other.clone(),
    }
}

pub fn unique(value: &Value) -> Value {
    match value {
        Value::List(items) => {
            let mut kept: Vec<Value> = Vec::with_capacity(items.len());
            for item in items {
                if !kept.iter().any(|found| found.equals(item)) {
                    kept.push(item.clone());
                }
            }
            Value::List(kept)
        }
        other => other.clone(),
    }
}

pub fn flat(value: &Value) -> Value {
    match value {
        Value::List(items) => Value::List(
            items
                .iter()
                .flat_map(super::items)
                .cloned()
                .collect(),
        ),
        other => other.clone(),
    }
}

pub fn slice(arguments: &[Value]) -> Value {
    let items = items(first(arguments));
    Value::List(items[span_of(arguments, items.len())].to_vec())
}

pub fn edge(value: &Value, leading: bool) -> Value {
    match value {
        Value::List(items) => {
            let found = if leading { items.first() } else { items.last() };
            found.cloned().unwrap_or(Value::Null)
        }
        other => other.clone(),
    }
}

pub fn nonnull(value: &Value) -> Value {
    match value {
        Value::List(items) => Value::List(
            items.iter().filter(|item| !matches!(item, Value::Null)).cloned().collect(),
        ),
        other => other.clone(),
    }
}
