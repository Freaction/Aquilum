use super::error::SearchError;
use super::query::{self, Found, Request};
use super::schema;
use std::path::Path;
use std::sync::RwLock;
use tantivy::schema::{Field, Value};
use tantivy::{Index, IndexReader, TantivyDocument};

pub fn stored_text(document: &TantivyDocument, field: Field) -> Option<String> {
    document.get_first(field).and_then(|value| value.as_str()).map(ToOwned::to_owned)
}

#[derive(Clone, Copy)]
pub struct SearchFields {
    pub id: Field,
    pub path: Field,
    pub title: Field,
    pub body: Field,
}

pub struct SearchIndex {
    pub index: Index,
    pub reader: IndexReader,
    pub fields: SearchFields,
    field_stats: RwLock<Option<FieldStats>>,
}

#[derive(Clone, Copy)]
pub struct FieldStats {
    pub average_title_length: f64,
    pub average_body_length: f64,
}

impl SearchIndex {
    pub fn open(directory: &Path) -> Result<Self, SearchError> {
        std::fs::create_dir_all(directory)?;
        let index = if directory.join("meta.json").is_file() {
            Index::open_in_dir(directory)?
        } else {
            Index::create_in_dir(directory, schema::build())?
        };
        schema::register_tokenizer(&index);
        let fields = schema::fields(&index.schema())?;
        let reader = schema::reader(&index)?;
        Ok(Self {
            index,
            reader,
            fields,
            field_stats: RwLock::new(None),
        })
    }

    pub fn document_count(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    pub fn reload(&self) -> Result<u64, SearchError> {
        self.reader.reload()?;
        *self.field_stats.write().map_err(|error| SearchError::Task {
            message: error.to_string(),
        })? = None;
        Ok(self.document_count())
    }

    pub fn field_stats(&self) -> Result<FieldStats, SearchError> {
        if let Some(stats) = self.field_stats.read().map_err(|error| SearchError::Task {
            message: error.to_string(),
        })?.as_ref().copied() {
            return Ok(stats);
        }

        let searcher = self.reader.searcher();
        let mut documents = 0u64;
        let mut title_length = 0u64;
        let mut body_length = 0u64;
        for segment in searcher.segment_readers() {
            let title_norms = segment.get_fieldnorms_reader(self.fields.title)?;
            let body_norms = segment.get_fieldnorms_reader(self.fields.body)?;
            for doc_id in segment.doc_ids_alive() {
                documents += 1;
                title_length += title_norms.fieldnorm(doc_id) as u64;
                body_length += body_norms.fieldnorm(doc_id) as u64;
            }
        }
        let divisor = documents.max(1) as f64;
        let stats = FieldStats {
            average_title_length: title_length as f64 / divisor,
            average_body_length: body_length as f64 / divisor,
        };
        *self.field_stats.write().map_err(|error| SearchError::Task {
            message: error.to_string(),
        })? = Some(stats);
        Ok(stats)
    }

    pub fn search(&self, request: &Request) -> Result<Found, SearchError> {
        query::search(self, request)
    }
}
