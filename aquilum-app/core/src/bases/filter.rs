use super::property::{string_literal, values_equal, Property};
use super::{BaseError, BaseRow};
use serde_json::Value;
use serde_yaml_ng::Value as Yaml;

pub(super) enum Filter {
    Equal(Property, Value),
    Tag(String),
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Vec<Filter>),
}

impl Filter {
    pub(super) fn parse(value: &Yaml) -> Result<Self, BaseError> {
        let invalid = || BaseError::UnsupportedFilter(format!("{value:?}"));
        match value {
            Yaml::String(expression) => Self::expression(expression),
            Yaml::Mapping(mapping) if mapping.len() == 1 => {
                let (operator, values) = mapping.iter().next().unwrap();
                let filters = values
                    .as_sequence()
                    .ok_or_else(invalid)?
                    .iter()
                    .map(Self::parse)
                    .collect::<Result<Vec<_>, _>>()?;
                match operator.as_str() {
                    Some("and") => Ok(Self::And(filters)),
                    Some("or") => Ok(Self::Or(filters)),
                    Some("not") => Ok(Self::Not(filters)),
                    _ => Err(invalid()),
                }
            }
            _ => Err(invalid()),
        }
    }

    fn expression(expression: &str) -> Result<Self, BaseError> {
        let raw = expression.trim();
        let invalid = || BaseError::UnsupportedFilter(expression.to_owned());
        if let Some(argument) = raw
            .strip_prefix("file.hasTag(")
            .and_then(|raw| raw.strip_suffix(')'))
        {
            let tag = string_literal(argument).ok_or_else(invalid)?;
            let tag = tag.trim_start_matches('#').to_owned();
            return if tag.is_empty() {
                Err(invalid())
            } else {
                Ok(Self::Tag(tag))
            };
        }
        let index = equality_index(raw).ok_or_else(invalid)?;
        let property = Property::parse(&raw[..index])?;
        let literal = raw[index + 2..].trim();
        let value = if let Some(text) = string_literal(literal) {
            Value::String(text)
        } else {
            let value: Value = serde_json::from_str(literal).map_err(|_| invalid())?;
            if !matches!(value, Value::Null | Value::Bool(_) | Value::Number(_)) {
                return Err(invalid());
            }
            value
        };
        Ok(Self::Equal(property, value))
    }

    pub(super) fn matches(&self, row: &BaseRow) -> bool {
        match self {
            Self::Equal(property, value) => values_equal(&property.value(row), value),
            Self::Tag(wanted) => {
                let wanted = wanted.to_lowercase();
                let carries = |tag: &String| {
                    let tag = tag.trim_start_matches('#').to_lowercase();
                    tag == wanted
                        || tag
                            .strip_prefix(wanted.as_str())
                            .is_some_and(|suffix| suffix.starts_with('/'))
                };
                row.file_tags.iter().any(carries)
            }
            Self::And(filters) => filters.iter().all(|filter| filter.matches(row)),
            Self::Or(filters) => filters.iter().any(|filter| filter.matches(row)),
            Self::Not(filters) => !filters.iter().any(|filter| filter.matches(row)),
        }
    }
}

fn equality_index(raw: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if raw[index..].starts_with("==") {
            return Some(index);
        }
    }
    None
}
