use super::filter::Filter;
use super::property::{compare_numbers, values_equal, Property};
use super::{BaseColumn, BaseDefinition, BaseDirection, BaseError, BaseRow};
use serde_json::Value;
use std::cmp::Ordering;

pub struct BaseRows;

impl BaseRows {
    pub fn evaluate(
        definition: &BaseDefinition,
        view_index: usize,
        notes: &[BaseRow],
    ) -> Result<Vec<BaseColumn>, BaseError> {
        let view = definition
            .views
            .get(view_index)
            .ok_or(BaseError::InvalidView(view_index))?;
        let filters = [definition.filters.as_ref(), view.filters.as_ref()]
            .into_iter()
            .flatten()
            .map(Filter::parse)
            .collect::<Result<Vec<_>, _>>()?;
        let grouping = view
            .group_by
            .as_ref()
            .map(|group| Property::parse(&group.property))
            .transpose()?;
        let sort = view
            .sort
            .iter()
            .map(|sort| Ok((Property::parse(&sort.property)?, sort.direction)))
            .collect::<Result<Vec<_>, BaseError>>()?;
        let mut columns = Vec::<BaseColumn>::new();
        if let Some(order) = &view.group_order {
            for value in order {
                validate_scalar(value, "groupOrder")?;
                let value = group_value(value.clone());
                if !columns
                    .iter()
                    .any(|column| values_equal(&column.value, &value))
                {
                    columns.push(BaseColumn {
                        value,
                        rows: Vec::new(),
                    });
                }
            }
        }
        let mut rows = Vec::new();
        for row in notes {
            if !filters.iter().all(|filter| filter.matches(row)) {
                continue;
            }
            let value = match &grouping {
                Some(property) => property.value(row),
                None => Value::Null,
            };
            validate_scalar(
                &value,
                view.group_by.as_ref().map_or("", |group| &group.property),
            )?;
            let value = group_value(value);
            if view.group_order.is_some()
                && !columns
                    .iter()
                    .any(|column| values_equal(&column.value, &value))
            {
                continue;
            }
            let keys = sort
                .iter()
                .map(|(property, _)| property.value(row))
                .collect::<Vec<_>>();
            for (key, setting) in keys.iter().zip(&view.sort) {
                validate_scalar(key, &setting.property)?;
            }
            rows.push((row, value, keys));
        }
        rows.sort_by(|left, right| {
            for ((left, right), (_, direction)) in left.2.iter().zip(&right.2).zip(&sort) {
                let ordering = compare_values(left, right);
                let ordering = match direction {
                    BaseDirection::Asc => ordering,
                    BaseDirection::Desc => ordering.reverse(),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            Ordering::Equal
        });
        for (row, value, _) in rows.into_iter().take(view.limit.unwrap_or(usize::MAX)) {
            if let Some(column) = columns
                .iter_mut()
                .find(|column| values_equal(&column.value, &value))
            {
                column.rows.push(row.clone());
            } else {
                columns.push(BaseColumn {
                    value,
                    rows: vec![row.clone()],
                });
            }
        }
        if view.group_order.is_none() {
            let direction = view
                .group_by
                .as_ref()
                .map_or(BaseDirection::Asc, |group| group.direction);
            columns.sort_by(|left, right| match (&left.value, &right.value) {
                (Value::Null, Value::Null) => Ordering::Equal,
                (Value::Null, _) => Ordering::Greater,
                (_, Value::Null) => Ordering::Less,
                _ => match direction {
                    BaseDirection::Asc => compare_values(&left.value, &right.value),
                    BaseDirection::Desc => compare_values(&right.value, &left.value),
                },
            });
        }
        Ok(columns)
    }
}

fn group_value(value: Value) -> Value {
    if value.as_str().is_some_and(str::is_empty) {
        Value::Null
    } else {
        value
    }
}

fn validate_scalar(value: &Value, property: &str) -> Result<(), BaseError> {
    match value {
        Value::Array(_) | Value::Object(_) => Err(BaseError::InvalidGrouping(property.to_owned())),
        _ => Ok(()),
    }
}

fn compare_values(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Less,
        (_, Value::Null) => Ordering::Greater,
        (Value::Number(left), Value::Number(right)) => compare_numbers(left, right),
        (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
        (Value::String(left), Value::String(right)) => left.cmp(right),
        _ => left.to_string().cmp(&right.to_string()),
    }
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;

#[cfg(test)]
mod scalar_validation_tests {
    use super::{BaseDefinition, BaseRow, BaseRows};
    use serde_json::json;

    #[test]
    fn rejects_complex_sort_values_with_context_neutral_error() {
        let definition =
            BaseDefinition::parse("views: [{type: kanban, sort: [{property: rank}]}]").unwrap();
        for value in [json!([1, 2]), json!({"value": 1})] {
            let row = BaseRow {
                path: "a.md".to_owned(),
                file_tags: Vec::new(),
                fields: json!({"rank": value}).as_object().unwrap().clone(),
            };
            let error = BaseRows::evaluate(&definition, 0, &[row]).unwrap_err();
            assert_eq!(
                error.to_string(),
                "Base property requires a scalar value: rank"
            );
        }
    }
}
