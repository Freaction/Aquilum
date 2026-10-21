mod count;
mod excerpt;
mod file;
mod jump;

use super::error::SearchError;
use super::index::{stored_text, SearchFields};
use super::models::SearchResult;
use super::pages;
use rusqlite::Connection;
use crate::text::cancel::Cancel;
use crate::text::matcher::{Fold, Matcher, Mode, Options, EXACT};
use file::{locate, Located};
use std::path::Path;
use tantivy::{DocAddress, Searcher, TantivyDocument};

pub const QUERY_MATCH: Options = Options { fold: Fold::Ascii, mode: Mode::WordPrefix, ..EXACT };

pub fn query_matcher<S: AsRef<str>>(terms: &[S]) -> Matcher {
    Matcher::any(terms, QUERY_MATCH)
}

pub fn group(searcher: &Searcher, fields: SearchFields, addresses: impl IntoIterator<Item = DocAddress>) -> Result<Vec<String>, SearchError> {
    let mut notes: Vec<String> = Vec::new();
    for address in addresses {
        let document = searcher.doc::<TantivyDocument>(address)?;
        if let Some(path) = stored_text(&document, fields.id).filter(|path| !notes.contains(path)) {
            notes.push(path);
        }
    }
    Ok(notes)
}

pub fn results(searcher: &Searcher, fields: SearchFields, terms: &[String], notes: &[String], metadata: Option<&Path>, cancel: &Cancel) -> Result<Vec<SearchResult>, SearchError> {
    let matcher = query_matcher(terms);
    let connection = metadata.and_then(|metadata| Connection::open(metadata).ok());
    let mut results = Vec::with_capacity(notes.len());
    for note in notes {
        let hits = count::note_hits(searcher, fields, note, terms)?;
        let path = Path::new(note);
        let placed = || connection.as_ref().and_then(|connection| pages::placed(connection, note).ok());
        let first = if hits.pages.is_empty() {
            None
        } else {
            placed().and_then(|placed| jump::locate(path, &placed, &hits.pages, &matcher)).or_else(|| locate(path, Some((&matcher, &hits.pages)), cancel).ok().flatten())
        };
        if cancel.is_cancelled() {
            return Err(SearchError::Cancelled);
        }
        let shown = first.or_else(|| locate(path, None, cancel).ok().flatten());
        results.push(result(note, terms, &matcher, &hits.counts, shown));
    }
    Ok(results)
}

fn result(path: &str, terms: &[String], matcher: &Matcher, body: &[u32], first: Option<Located>) -> SearchResult {
    let file = Path::new(path);
    let title = crate::search::paths::title(file);
    let file_name = file.file_name().and_then(|value| value.to_str()).unwrap_or(&title);
    let match_count = matcher.find_all(file_name).len() + count::total(terms, body);
    let matched_terms = terms
        .iter()
        .zip(body)
        .filter(|(term, &count)| count > 0 || query_matcher(&[term]).first(file_name).is_some())
        .map(|(term, _)| term.clone())
        .collect();
    let (match_offset, snippet, heading) = match first {
        Some(found) => (found.utf16, found.snippet, found.heading),
        None => (0, String::new(), None),
    };
    SearchResult {
        extension: file.extension().and_then(|value| value.to_str()).unwrap_or_default().to_owned(),
        match_offset,
        snippet,
        heading,
        match_count,
        matched_terms,
        path: path.to_owned(),
        title,
    }
}
