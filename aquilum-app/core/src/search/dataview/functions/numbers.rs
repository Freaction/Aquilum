use super::{first, items};
use super::super::value::Value;

pub fn numeric(arguments: &[Value], apply: fn(f64) -> f64) -> Value {
    match first(arguments).as_number() {
        Some(number) => Value::Number(apply(number)),
        None => Value::Null,
    }
}

pub fn numbers(value: &Value) -> impl Iterator<Item = f64> + '_ {
    items(value).iter().filter_map(Value::as_number)
}

pub fn extreme(value: &Value, smallest: bool) -> Value {
    items(value)
        .iter()
        .filter(|item| !matches!(item, Value::Null))
        .reduce(|left, right| {
            let take_right = if smallest {
                right.compare(&left).is_lt()
            } else {
                right.compare(&left).is_gt()
            };
            if take_right {
                right
            } else {
                left
            }
        })
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn average(value: &Value) -> Value {
    let (sum, count) = numbers(value).fold((0.0, 0usize), |(sum, count), number| (sum + number, count + 1));
    if count == 0 {
        return Value::Null;
    }
    Value::Number(sum / count as f64)
}
