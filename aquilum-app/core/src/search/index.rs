use super::error::SearchError;
use super::query::{self, Found, Request};
use super::schema;
use std::path::Path;
use tantivy::schema::Field;
use tantivy::{Index, IndexReader, Searcher};

#[derive(Clone, Copy)]
pub struct SearchFields {
    pub id: Field,
    pub title: Field,
    pub headings: Field,
    pub body: Field,
    pub page: Field,
}

pub struct SearchIndex {
    pub index: Index,
    pub reader: IndexReader,
    pub fields: SearchFields,
}

#[derive(Clone, Copy)]
pub struct FieldStats {
    pub average_title_length: f64,
    pub average_body_length: f64,
}

impl FieldStats {
    pub fn of(searcher: &Searcher, fields: SearchFields) -> Result<Self, SearchError> {
        let (mut title, mut body) = (0, 0);
        for segment in searcher.segment_readers() {
            title += segment.inverted_index(fields.title)?.total_num_tokens();
            body += segment.inverted_index(fields.body)?.total_num_tokens();
        }
        let documents = searcher.num_docs().max(1) as f64;
        Ok(Self { average_title_length: title as f64 / documents, average_body_length: body as f64 / documents })
    }
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
        Ok(Self { index, reader, fields })
    }

    pub fn document_count(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    pub fn reload(&self) -> Result<u64, SearchError> {
        self.reader.reload()?;
        Ok(self.document_count())
    }

    pub fn search(&self, request: &Request) -> Result<Found, SearchError> {
        query::search(self, request)
    }
}
