pub mod inline;
pub mod parse;
pub mod query;
pub mod repository;
pub mod service;
pub mod value;

pub use parse::note_fields;
pub use query::{candidates, paths_with_tag, read as read_fields};
pub use inline::page_fields;
pub use repository::{store, FIELDS_DDL};
pub use value::{Field, FieldKind};

pub type NoteFieldRows = Vec<(String, Vec<Field>)>;

use serde::Serialize;
use serde_json::{Map, Value};
use std::path::Path;

pub fn head_fields(path: &Path) -> Vec<Field> {
    crate::files::document::read_head(path).map(|head| note_fields(&head)).unwrap_or_default()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteFields {
    pub path: String,
    pub fields: Map<String, Value>,
}

pub fn as_object(found: &[Field]) -> Map<String, Value> {
    found
        .iter()
        .map(|field| (field.key.clone(), field.json()))
        .collect()
}
