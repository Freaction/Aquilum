mod count;
mod excerpt;
mod file;

use super::error::SearchError;
use super::headings::heading_at;
use super::index::{stored_text, SearchFields};
use super::models::SearchResult;
use crate::text::cancel::Cancel;
use crate::text::matcher::{Fold, Matcher, Mode, Options, EXACT};
use excerpt::{excerpt, CHARS_AFTER_MATCH};
use file::{first_match, read_through, FirstMatch};
use std::path::Path;
use tantivy::{DocAddress, Searcher, TantivyDocument};

pub const QUERY_MATCH: Options = Options { fold: Fold::Ascii, mode: Mode::WordPrefix, ..EXACT };

pub fn query_matcher<S: AsRef<str>>(terms: &[S]) -> Matcher {
    Matcher::any(terms, QUERY_MATCH)
}

pub fn results(
    searcher: &Searcher,
    fields: SearchFields,
    terms: &[String],
    addresses: &[DocAddress],
    cancel: &Cancel,
) -> Result<Vec<SearchResult>, SearchError> {
    let counts = count::body_counts(searcher, fields.body, terms, addresses)?;
    let matcher = query_matcher(terms);
    let mut results = Vec::with_capacity(addresses.len());
    for (address, body) in addresses.iter().zip(counts) {
        let document = searcher.doc::<TantivyDocument>(*address)?;
        let Some(path) = stored_text(&document, fields.id) else {
            continue;
        };
        let title = stored_text(&document, fields.title).unwrap_or_default();
        let first = match body.iter().any(|&count| count > 0) {
            true => first_match(Path::new(&path), &matcher, cancel).ok().flatten(),
            false => None,
        };
        if cancel.is_cancelled() {
            return Err(SearchError::Cancelled);
        }
        results.push(result(path, title, terms, &matcher, &body, first));
    }
    Ok(results)
}

fn result(path: String, title: String, terms: &[String], matcher: &Matcher, body: &[u32], first: Option<FirstMatch>) -> SearchResult {
    let file = Path::new(&path);
    let file_name = file.file_name().and_then(|value| value.to_str()).unwrap_or(&title);
    let match_count = matcher.find_all(file_name).len() + count::total(terms, body);
    let matched_terms = terms
        .iter()
        .zip(body)
        .filter(|(term, &count)| count > 0 || query_matcher(&[term]).first(file_name).is_some())
        .map(|(term, _)| term.clone())
        .collect();
    let wanted = first.as_ref().map_or(0, |found| found.byte);
    let (text, ended) = read_through(file, wanted, CHARS_AFTER_MATCH * 4).unwrap_or_else(|_| (String::new(), true));
    let at = (0..=wanted.min(text.len())).rev().find(|&at| text.is_char_boundary(at)).unwrap_or(0);
    SearchResult {
        extension: file.extension().and_then(|value| value.to_str()).unwrap_or_default().to_owned(),
        snippet: excerpt(&text, at, ended),
        match_count,
        match_offset: first.as_ref().map_or(0, |found| found.utf16),
        heading: first.and_then(|_| heading_at(&text, at)),
        matched_terms,
        path,
        title,
    }
}
