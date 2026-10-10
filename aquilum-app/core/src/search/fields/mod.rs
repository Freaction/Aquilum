#[cfg(test)]
mod base_service_tests;
mod bases;
pub mod inline;
pub mod parse;
pub mod query;
pub mod repository;
pub mod service;
pub mod value;

pub use parse::note_fields;
pub use query::{candidates, paths_with_tag, read as read_fields};
pub use repository::{index_document, open_schema, remove_document};
pub use value::{Field, FieldKind};

pub type NoteFieldRows = Vec<(String, Vec<Field>)>;

use serde::Serialize;
use serde_json::{Map, Value};
use std::io::Read;
use std::path::Path;

const HEAD_BYTES: u64 = 8_192;

pub fn head_fields(path: &Path) -> Vec<Field> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut buffer = Vec::new();
    if file.take(HEAD_BYTES).read_to_end(&mut buffer).is_err() {
        return Vec::new();
    }
    let head = String::from_utf8_lossy(&buffer).into_owned();
    note_fields(&crate::files::document::normalize_line_endings(head))
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
