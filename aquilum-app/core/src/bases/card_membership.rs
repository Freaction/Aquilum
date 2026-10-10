use super::card_metadata;
use super::filter::Filter;
use super::property::{values_equal, Property};
use super::{note, BaseCardError, BaseDefinition, BaseError, BaseRow};
use serde_json::Value;

pub(super) struct Membership {
    pub(super) group: String,
    filter: Option<Filter>,
    order: Option<Vec<Value>>,
}

impl BaseDefinition {
    pub fn can_mutate_group(&self, view_index: usize) -> bool {
        Membership::new(self, view_index).is_ok()
    }
}

impl Membership {
    pub(super) fn new(
        definition: &BaseDefinition,
        view_index: usize,
    ) -> Result<Self, BaseCardError> {
        let view = definition
            .views
            .get(view_index)
            .ok_or(BaseError::InvalidView(view_index))?;
        if let Some(order) = &view.group_order {
            for value in order {
                scalar(value, "groupOrder")?;
            }
        }
        let grouping = view
            .group_by
            .as_ref()
            .ok_or_else(|| BaseCardError::Invalid("A grouping property is required".into()))?;
        let Property::Note(group) = Property::parse(&grouping.property)? else {
            return Err(BaseError::InvalidGrouping(grouping.property.clone()).into());
        };
        if group == "tags" {
            return Err(BaseError::InvalidGrouping(group).into());
        }
        let mut filters = [definition.filters.as_ref(), view.filters.as_ref()]
            .into_iter()
            .flatten();
        let filter = filters.next().map(Filter::parse).transpose()?;
        if filters.next().is_some() {
            return Err(BaseCardError::Invalid(
                "Multiple membership filters cannot be mutated safely".into(),
            ));
        }
        match &filter {
            None | Some(Filter::Tag(_)) => {}
            Some(Filter::Equal(Property::Note(key), _)) if *key != group && key != "tags" => {}
            _ => {
                return Err(BaseCardError::Invalid(
                    "Membership must be a single independent note equality or tag".into(),
                ))
            }
        }
        Ok(Self {
            group,
            filter,
            order: view.group_order.clone(),
        })
    }

    pub(super) fn row(&self, path: String, text: &str) -> Result<BaseRow, BaseCardError> {
        let fields = note::fields(text)?;
        let value = fields.get(&self.group).unwrap_or(&Value::Null);
        scalar(value, &self.group)?;
        let file_tags = note::tags(text, &fields);
        Ok(BaseRow {
            path,
            fields,
            file_tags,
        })
    }

    pub(super) fn update(
        &self,
        text: &str,
        group: &Value,
        creating: bool,
    ) -> Result<String, BaseCardError> {
        scalar(group, &self.group)?;
        if self.order.as_ref().is_some_and(|order| {
            !order
                .iter()
                .any(|value| values_equal(&unassigned(value), &unassigned(group)))
        }) {
            return Err(BaseCardError::Invalid(
                "The target column is hidden by groupOrder".into(),
            ));
        }
        let mut text = text.to_owned();
        if creating {
            match &self.filter {
                Some(Filter::Equal(Property::Note(key), value)) => {
                    text = card_metadata::set(&text, key, value)?
                }
                Some(Filter::Tag(tag)) => {
                    let fields = note::fields(&text)?;
                    let mut tags = match fields.get("tags") {
                        None | Some(Value::Null) => Vec::new(),
                        Some(Value::String(tag)) => vec![Value::String(tag.clone())],
                        Some(Value::Array(tags)) if tags.iter().all(Value::is_string) => {
                            tags.clone()
                        }
                        _ => return Err(BaseCardError::Invalid("Tags must be strings".into())),
                    };
                    if !tags.iter().any(|value| {
                        value.as_str().is_some_and(|value| {
                            value.trim_start_matches('#').to_lowercase() == tag.to_lowercase()
                        })
                    }) {
                        tags.push(Value::String(tag.clone()));
                    }
                    text = card_metadata::set(&text, "tags", &Value::Array(tags))?;
                }
                _ => {}
            }
        }
        Ok(card_metadata::set(&text, &self.group, group)?)
    }

    pub(super) fn matches(&self, row: &BaseRow) -> bool {
        self.filter
            .as_ref()
            .is_none_or(|filter| filter.matches(row))
    }
}

pub(super) fn scalar(value: &Value, property: &str) -> Result<(), BaseError> {
    if value.is_array() || value.is_object() {
        Err(BaseError::InvalidGrouping(property.to_owned()))
    } else {
        Ok(())
    }
}

pub(super) fn unassigned(value: &Value) -> Value {
    if value.as_str() == Some("") {
        Value::Null
    } else {
        value.clone()
    }
}
