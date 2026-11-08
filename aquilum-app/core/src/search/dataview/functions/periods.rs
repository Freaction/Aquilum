use super::dates::{as_date, date};
use super::{first, items};
use super::super::constants::{DEFAULT_DASH, DEFAULT_ZERO_DELTA};
use super::super::duration;
use super::super::span;
use super::super::value::Value;

pub fn time_span(arguments: &[Value]) -> Value {
    let start = as_date(first(arguments));
    let end = as_date(arguments.get(1).unwrap_or(&Value::Null));
    match (start, end) {
        (Some(start), Some(end)) => span::of(start, end),
        _ => Value::Null,
    }
}

pub fn bar(value: &Value) -> Value {
    match span::passed(value) {
        Some(part) => Value::Progress(part),
        None => match value.as_number() {
            Some(number) => Value::Progress(number.clamp(0.0, 100.0)),
            None => Value::Null,
        },
    }
}

pub fn percent(value: &Value) -> Value {
    match span::passed(value) {
        Some(part) => Value::Number((part * 10.0).round() / 10.0),
        None => Value::Null,
    }
}

pub fn remaining(value: &Value) -> Value {
    match span::left(value) {
        Some(nanos) => Value::Duration(duration::Duration { months: 0, nanos }),
        None => Value::Null,
    }
}

pub fn within(arguments: &[Value]) -> Value {
    let moment = match date(first(arguments)) {
        Value::Date(nanos) => nanos,
        _ => return Value::Bool(false),
    };
    let period = arguments.get(1).unwrap_or(&Value::Null);
    Value::Bool(span::covers(period, moment).unwrap_or(false))
}

pub fn count(arguments: &[Value]) -> Value {
    if arguments.len() > 1 && matches!(arguments.get(1), Some(Value::Null)) {
        return Value::Null;
    }
    let period = arguments.get(1);
    let moments = items(first(arguments));
    if let Some(period) = period {
        let found = moments
            .iter()
            .filter(|moment| match date(moment) {
                Value::Date(nanos) => span::covers(period, nanos).unwrap_or(false),
                _ => false,
            })
            .count();
        Value::Number(found as f64)
    } else {
        Value::Number(moments.iter().filter(|moment| !matches!(moment, Value::Null)).count() as f64)
    }
}

pub fn delta(arguments: &[Value]) -> Value {
    let dash = arguments
        .get(2)
        .map(Value::text)
        .unwrap_or_else(|| DEFAULT_DASH.to_owned());
    let (Some(current), Some(before)) = (
        first(arguments).as_number(),
        arguments.get(1).and_then(Value::as_number),
    ) else {
        return Value::Text(dash);
    };
    let difference = current - before;
    if difference == 0.0 {
        if arguments.len() >= 3 {
            return Value::Text(dash);
        }
        return Value::Text(DEFAULT_ZERO_DELTA.to_owned());
    }
    let sign = if difference > 0.0 { "+" } else { "" };
    Value::Text(format!("{sign}{}", Value::Number(difference).text()))
}
