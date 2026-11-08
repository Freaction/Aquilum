use crate::search::paths::strip_markdown_extension;
use super::duration::Duration;
use crate::search::note_date;
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub target: String,
    pub display: Option<String>,
}

impl Link {
    pub fn label(&self) -> String {
        self.display.clone().unwrap_or_else(|| {
            let name = self.target.rsplit('/').next().unwrap_or(&self.target);
            strip_markdown_extension(name).to_owned()
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Date(i64),
    Duration(Duration),
    Progress(f64),
    Text(String),
    Link(Link),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn text(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => format_number(*value),
            Value::Date(nanos) => note_date::format(*nanos),
            Value::Duration(duration) => duration.text(),
            Value::Progress(part) => format!("{}%", format_number((part * 10.0).round() / 10.0)),
            Value::Text(value) => value.clone(),
            Value::Link(link) => link.label(),
            Value::List(items) => items
                .iter()
                .map(Value::text)
                .collect::<Vec<_>>()
                .join(", "),
            Value::Object(fields) => fields
                .iter()
                .map(|(key, value)| format!("{key}: {}", value.text()))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(value) => *value,
            Value::Number(value) => *value != 0.0,
            Value::Date(nanos) => *nanos != 0,
            Value::Duration(duration) => !duration.is_zero(),
            Value::Progress(part) => *part != 0.0,
            Value::Text(value) => !value.is_empty(),
            Value::Link(link) => !link.target.is_empty(),
            Value::List(items) => !items.is_empty(),
            Value::Object(fields) => !fields.is_empty(),
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(value) => Some(*value),
            Value::Progress(part) => Some(*part),
            Value::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Value::Text(value) => value.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::Number(left), Value::Number(right)) => {
                left.partial_cmp(right).unwrap_or(Ordering::Equal)
            }
            (Value::Date(left), Value::Date(right)) => left.cmp(right),
            (Value::Duration(left), Value::Duration(right)) => {
                left.approximate_nanos().cmp(&right.approximate_nanos())
            }
            (Value::Progress(left), Value::Progress(right)) => {
                left.partial_cmp(right).unwrap_or(Ordering::Equal)
            }
            (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
            (Value::Text(left), Value::Text(right)) => compare_text(left, right),
            (Value::Link(left), Value::Link(right)) => compare_text(&left.label(), &right.label()),
            (Value::List(left), Value::List(right)) => {
                for (left, right) in left.iter().zip(right.iter()) {
                    let ordering = left.compare(right);
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                }
                left.len().cmp(&right.len())
            }
            _ => rank(self).cmp(&rank(other)),
        }
    }

    pub fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Null, _) | (_, Value::Null) => false,
            (Value::Number(left), Value::Number(right)) => left == right,
            (Value::Date(left), Value::Date(right)) => left == right,
            (Value::Duration(left), Value::Duration(right)) => left == right,
            (Value::List(left), Value::List(right)) => {
                left.len() == right.len()
                    && left.iter().zip(right.iter()).all(|(l, r)| l.equals(r))
            }
            _ => {
                let (left, right) = (self.text(), other.text());
                left.eq_ignore_ascii_case(&right) || left.to_lowercase() == right.to_lowercase()
            }
        }
    }
}

fn rank(value: &Value) -> u8 {
    match value {
        Value::Number(_) => 0,
        Value::Progress(_) => 0,
        Value::Date(_) => 1,
        Value::Duration(_) => 2,
        Value::Bool(_) => 3,
        Value::Text(_) => 4,
        Value::Link(_) => 5,
        Value::List(_) => 6,
        Value::Object(_) => 7,
        Value::Null => 8,
    }
}

fn compare_text(left: &str, right: &str) -> Ordering {
    let folded = left.to_lowercase().cmp(&right.to_lowercase());
    if folded == Ordering::Equal {
        left.cmp(right)
    } else {
        folded
    }
}

fn format_number(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let text = format!("{value}");
        text
    }
}

#[cfg(test)]
mod tests {
    use super::{Link, Value};
    use std::cmp::Ordering;

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    #[test]
    fn an_integer_keeps_no_decimal_tail() {
        assert_eq!(Value::Number(5.0).text(), "5");
        assert_eq!(Value::Number(4.5).text(), "4.5");
    }

    #[test]
    fn a_link_without_a_label_shows_the_note_name() {
        let link = Link { target: "Книги/Пороки.md".to_owned(), display: None };
        assert_eq!(Value::Link(link).text(), "Пороки");
    }

    #[test]
    fn a_link_label_wins_when_it_is_set() {
        let link = Link {
            target: "Книги/Пороки.md".to_owned(),
            display: Some("Пять пороков".to_owned()),
        };
        assert_eq!(Value::Link(link).text(), "Пять пороков");
    }

    #[test]
    fn empty_values_sort_last() {
        assert_eq!(text("а").compare(&Value::Null), Ordering::Less);
        assert_eq!(Value::Null.compare(&Value::Number(0.0)), Ordering::Greater);
    }

    #[test]
    fn text_compares_case_insensitively_but_stays_deterministic() {
        assert_eq!(text("Ант").compare(&text("бор")), Ordering::Less);
        assert_eq!(text("ант").compare(&text("Ант")), Ordering::Greater);
    }

    #[test]
    fn numbers_compare_as_numbers_not_as_text() {
        assert_eq!(Value::Number(10.0).compare(&Value::Number(5.0)), Ordering::Greater);
        assert_eq!(text("10").compare(&text("5")), Ordering::Less);
    }

    #[test]
    fn equality_ignores_case_and_type_of_a_scalar() {
        assert!(text("Read").equals(&text("read")));
        assert!(Value::Number(5.0).equals(&text("5")));
        assert!(Value::Bool(true).equals(&text("TRUE")));
        assert!(!text("read").equals(&Value::Null));
    }

    #[test]
    fn a_list_is_truthy_only_when_it_has_items() {
        assert!(!Value::List(Vec::new()).truthy());
        assert!(Value::List(vec![text("а")]).truthy());
    }
}
