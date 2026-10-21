use super::analyzer::indexing_tokenizer;
use super::error::SearchError;
use super::hits;
use super::index::SearchIndex;
use super::models::SearchResult;
use crate::text::cancel::Cancel;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, Query, QueryClone, TermQuery, TermSetQuery};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::tokenizer::TokenStream;
use tantivy::{Searcher, Term};

const MIN_FUZZY_CHARS: usize = 5;
const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;

pub struct Request<'a> {
    pub text: &'a str,
    pub folder: Option<&'a Path>,
    pub limit: Option<usize>,
    pub cancel: Cancel,
    pub metadata: Option<&'a Path>,
}

impl<'a> Request<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, folder: None, limit: None, cancel: Cancel::never(), metadata: None }
    }
}

pub struct Found {
    pub terms: Vec<String>,
    pub results: Vec<SearchResult>,
    pub has_more: bool,
}

pub fn analyzed_query_terms(input: &str) -> Vec<String> {
    let mut terms = Vec::new();
    indexing_tokenizer().token_stream(input).process(&mut |token| {
        if !token.text.is_empty() {
            terms.push(token.text.clone());
        }
    });
    terms.sort();
    terms.dedup();
    terms
}

pub fn search(index: &SearchIndex, request: &Request) -> Result<Found, SearchError> {
    let terms = analyzed_query_terms(request.text);
    if terms.is_empty() {
        return Ok(Found { terms, results: Vec::new(), has_more: false });
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let searcher = index.reader.searcher();
    let folder = request.folder.map(|folder| folder_query(&searcher, index.fields.id, folder)).transpose()?;
    let within = |query: BooleanQuery| {
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, Box::new(query))];
        clauses.extend(folder.iter().map(|folder| (Occur::Must, folder.box_clone())));
        BooleanQuery::new(clauses)
    };
    let mut notes = top_notes(&searcher, index, &within(combined_query(index, &terms, &[index.fields.title])), limit, &request.cancel)?;
    for note in top_notes(&searcher, index, &within(combined_query(index, &terms, &searchable_fields(index))), limit, &request.cancel)? {
        if !notes.contains(&note) {
            notes.push(note);
        }
    }
    let has_more = notes.len() > limit;
    notes.truncate(limit);
    let results = hits::results(&searcher, index.fields, &terms, &notes, request.metadata, &request.cancel)?;
    Ok(Found { terms, results, has_more })
}

fn top_notes(searcher: &Searcher, index: &SearchIndex, query: &BooleanQuery, limit: usize, cancel: &Cancel) -> Result<Vec<String>, SearchError> {
    let mut wanted = limit + 1;
    loop {
        let top = searcher.search(query, &TopDocs::with_limit(wanted).order_by_score())?;
        if cancel.is_cancelled() {
            return Err(SearchError::Cancelled);
        }
        let found = top.len();
        let notes = hits::group(searcher, index.fields, top.into_iter().map(|(_, address)| address))?;
        if notes.len() > limit || found < wanted {
            return Ok(notes);
        }
        wanted *= 4;
    }
}

fn folder_query(searcher: &Searcher, id: Field, folder: &Path) -> Result<TermSetQuery, SearchError> {
    let mut inside = Vec::new();
    for segment in searcher.segment_readers() {
        let inverted = segment.inverted_index(id)?;
        let mut paths = inverted.terms().stream()?;
        while paths.advance() {
            if let Ok(path) = std::str::from_utf8(paths.key()) {
                if Path::new(path).starts_with(folder) {
                    inside.push(Term::from_field_text(id, path));
                }
            }
        }
    }
    Ok(TermSetQuery::new(inside))
}

fn searchable_fields(index: &SearchIndex) -> [Field; 3] {
    [index.fields.title, index.fields.path, index.fields.body]
}

fn combined_query(index: &SearchIndex, terms: &[String], fields: &[Field]) -> BooleanQuery {
    let mut term_clauses = Vec::<(Occur, Box<dyn Query>)>::with_capacity(terms.len());
    let mut exact_clauses = Vec::<(Occur, Box<dyn Query>)>::new();

    for text in terms {
        let mut field_matches = Vec::<(Occur, Box<dyn Query>)>::with_capacity(fields.len());
        for &field in fields {
            let term = Term::from_field_text(field, text);
            let boost = field_boost(index, field);
            field_matches.push((
                Occur::Should,
                Box::new(BoostQuery::new(field_matcher(&term, text), boost)),
            ));
            exact_clauses.push((
                Occur::Should,
                Box::new(BoostQuery::new(
                    Box::new(TermQuery::new(term, IndexRecordOption::WithFreqs)),
                    boost * 2.0,
                )),
            ));
        }
        term_clauses.push((Occur::Should, Box::new(BooleanQuery::new(field_matches))));
    }

    let any_term = BooleanQuery::with_minimum_required_clauses(term_clauses, 1);
    BooleanQuery::new(vec![
        (Occur::Must, Box::new(any_term)),
        (Occur::Should, Box::new(BooleanQuery::new(exact_clauses))),
    ])
}

fn field_matcher(term: &Term, text: &str) -> Box<dyn Query> {
    let distance = u8::from(text.chars().count() >= MIN_FUZZY_CHARS);
    Box::new(FuzzyTermQuery::new_prefix(term.clone(), distance, true))
}

fn field_boost(index: &SearchIndex, field: Field) -> f32 {
    if field == index.fields.title {
        5.0
    } else if field == index.fields.path {
        2.0
    } else {
        1.0
    }
}
