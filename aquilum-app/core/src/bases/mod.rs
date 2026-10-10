mod card_membership;
mod card_metadata;
mod cards;
mod filter;
pub(crate) mod note;
mod parse;
mod property;
mod query;

#[cfg(test)]
mod cards_tests;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt;

pub use cards::BaseCardError;
pub use query::BaseRows;

#[derive(Clone, Debug, Serialize)]
pub struct BaseRow {
    pub path: String,
    pub fields: Map<String, Value>,
    pub file_tags: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BaseDefinition {
    #[serde(skip)]
    pub source: String,
    pub filters: Option<serde_yaml_ng::Value>,
    #[serde(default)]
    pub views: Vec<BaseView>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseView {
    #[serde(rename = "type")]
    pub view_type: String,
    #[serde(default)]
    pub name: String,
    pub filters: Option<serde_yaml_ng::Value>,
    pub group_by: Option<BaseGroupBy>,
    pub group_order: Option<Vec<Value>>,
    #[serde(default)]
    pub sort: Vec<BaseSort>,
    pub limit: Option<usize>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BaseGroupBy {
    pub property: String,
    #[serde(default)]
    pub direction: BaseDirection,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BaseSort {
    pub property: String,
    #[serde(default)]
    pub direction: BaseDirection,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum BaseDirection {
    #[default]
    #[serde(rename = "ASC")]
    Asc,
    #[serde(rename = "DESC")]
    Desc,
}

#[derive(Clone, Debug)]
pub struct BaseColumn {
    pub value: Value,
    pub rows: Vec<BaseRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseError {
    InvalidYaml(String),
    InvalidView(usize),
    UnsupportedFilter(String),
    UnsupportedProperty(String),
    InvalidGrouping(String),
}

impl fmt::Display for BaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidYaml(message) => write!(formatter, "Invalid Base YAML: {message}"),
            Self::InvalidView(index) => write!(formatter, "Base view {index} does not exist"),
            Self::UnsupportedFilter(filter) => {
                write!(formatter, "Unsupported Base filter: {filter}")
            }
            Self::UnsupportedProperty(property) => {
                write!(formatter, "Unsupported Base property: {property}")
            }
            Self::InvalidGrouping(property) => {
                write!(
                    formatter,
                    "Base property requires a scalar value: {property}"
                )
            }
        }
    }
}

impl std::error::Error for BaseError {}
