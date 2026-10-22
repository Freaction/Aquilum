use super::derive::{derive, Terms};
use super::parse::Parsed;
use crate::search::error::SearchError;
use crate::search::index::{FieldStats, SearchFields};
use std::path::Path;
use tantivy::query::{BooleanQuery, EnableScoring, Occur, PhraseQuery, Query, TermQuery, TermSetQuery, Weight};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{Searcher, Term};

pub struct Plan {
    pub fields: SearchFields,
    pub terms: Terms,
    pub required: Option<Box<dyn Weight>>,
    pub excluded: Option<Box<dyn Weight>>,
    pub words: u32,
    pub all: u32,
    pub documents: f64,
    pub average: f64,
}

pub fn plan(searcher: &Searcher, fields: SearchFields, parsed: &Parsed, folder: Option<&Path>) -> Result<Plan, SearchError> {
    let mut required: Vec<(Occur, Box<dyn Query>)> = parsed.phrases.iter().map(|stems| (Occur::Must, anywhere(fields, stems))).collect();
    if let Some(folder) = folder {
        required.push((Occur::Must, Box::new(folder_query(searcher, fields.id, folder)?)));
    }
    let excluded = parsed.excluded.iter().map(|stems| (Occur::Should, anywhere(fields, stems))).collect();
    let weight = |clauses: Vec<(Occur, Box<dyn Query>)>| -> Result<Option<Box<dyn Weight>>, SearchError> {
        match clauses.is_empty() {
            true => Ok(None),
            false => Ok(Some(BooleanQuery::new(clauses).weight(EnableScoring::disabled_from_searcher(searcher))?)),
        }
    };
    Ok(Plan {
        fields,
        terms: derive(searcher, fields, &parsed.words)?,
        required: weight(required)?,
        excluded: weight(excluded)?,
        words: parsed.words.iter().enumerate().filter(|(_, word)| word.required()).fold(0, |mask, (index, _)| mask | 1 << index),
        all: u32::MAX >> (32 - parsed.words.len()),
        documents: searcher.num_docs() as f64,
        average: FieldStats::of(searcher, fields)?.average_body_length,
    })
}

fn anywhere(fields: SearchFields, stems: &[String]) -> Box<dyn Query> {
    let clauses = [fields.title, fields.body].map(|field| {
        let mut terms: Vec<Term> = stems.iter().map(|stem| Term::from_field_text(field, stem)).collect();
        let query: Box<dyn Query> = match terms.len() {
            1 => Box::new(TermQuery::new(terms.remove(0), IndexRecordOption::Basic)),
            _ => Box::new(PhraseQuery::new(terms)),
        };
        (Occur::Should, query)
    });
    Box::new(BooleanQuery::new(clauses.into()))
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
