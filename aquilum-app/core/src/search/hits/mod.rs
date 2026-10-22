mod excerpt;
mod file;
mod jump;
mod postings;

use super::error::SearchError;
use super::index::SearchFields;
use super::models::SearchResult;
use super::pages;
use rusqlite::Connection;
use crate::text::cancel::Cancel;
use crate::text::matcher::{Fold, Matcher, Mode, Options, EXACT};
use file::locate;
use std::cell::OnceCell;
use std::collections::HashSet;
use std::path::Path;
use tantivy::schema::Value;
use tantivy::{DocAddress, Searcher, TantivyDocument};

pub use postings::note_pages;

pub const QUERY_MATCH: Options = Options { fold: Fold::Ascii, mode: Mode::WordPrefix, ..EXACT };

pub fn query_matcher<S: AsRef<str>>(terms: &[S]) -> Matcher {
    Matcher::any(terms, QUERY_MATCH)
}

pub fn note_path(searcher: &Searcher, fields: SearchFields, address: DocAddress) -> Result<Option<String>, SearchError> {
    let document = searcher.doc::<TantivyDocument>(address)?;
    Ok(document.get_first(fields.id).and_then(|value| value.as_str()).map(ToOwned::to_owned))
}

pub fn found(path: String, match_count: usize, matched_terms: Vec<String>) -> SearchResult {
    let file = Path::new(&path);
    SearchResult {
        extension: file.extension().and_then(|value| value.to_str()).unwrap_or_default().to_owned(),
        title: crate::search::paths::title(file),
        snippet: String::new(),
        match_offset: 0,
        heading: None,
        match_count,
        matched_terms,
        path,
    }
}

pub fn results(notes: Vec<(SearchResult, HashSet<String>, usize)>, terms: &[String], metadata: Option<&Path>, cancel: &Cancel) -> Result<Vec<SearchResult>, SearchError> {
    let matcher = query_matcher(terms);
    let connection = OnceCell::new();
    let mut results = Vec::with_capacity(notes.len());
    for (mut result, hits, count) in notes {
        let path = Path::new(&result.path);
        let placed = || match (count, hits.iter().next()) {
            (1, Some(key)) => Some(vec![pages::Placed { key: key.clone(), start: 0, start_utf16: 0 }]),
            _ => connection.get_or_init(|| metadata.and_then(|metadata| Connection::open(metadata).ok())).as_ref().and_then(|connection| pages::placed(connection, &result.path).ok()),
        };
        let first = if hits.is_empty() {
            None
        } else {
            placed().and_then(|placed| jump::locate(path, &placed, &hits, &matcher)).or_else(|| locate(path, Some((&matcher, &hits)), cancel).ok().flatten())
        };
        if cancel.is_cancelled() {
            return Err(SearchError::Cancelled);
        }
        if let Some(found) = first.or_else(|| locate(path, None, cancel).ok().flatten()) {
            (result.match_offset, result.snippet, result.heading) = (found.utf16, found.snippet, found.heading);
        }
        results.push(result);
    }
    Ok(results)
}
