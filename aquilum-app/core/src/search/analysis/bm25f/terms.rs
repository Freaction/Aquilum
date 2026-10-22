use crate::search::analyzer::stem_surface_labels;
use crate::search::error::SearchError;
use crate::search::index::SearchIndex;
use crate::search::schema;
use crate::settings::models::Bm25fParams;
use std::cmp::Ordering;
use tantivy::query::{BooleanQuery, BoostQuery, Occur, Query, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::{Searcher, Term};

const IDF_PREFILTER_MULT: usize = 4;
const IDF_PREFILTER_FLOOR: usize = 256;

pub struct QueryTerm {
    pub text: String,
    pub label: String,
    pub idf: f64,
    pub frequency: u32,
    pub weight: f64,
}

pub fn select_terms(
    searcher: &Searcher,
    index: &SearchIndex,
    body: &str,
    title: &str,
    max_query_terms: usize,
) -> Result<Vec<QueryTerm>, SearchError> {
    let max_query_terms = max_query_terms.clamp(8, 128);
    let labels = stem_surface_labels(title, body);
    let mut frequencies = schema::term_frequencies(body);
    for (term, frequency) in schema::term_frequencies(title) {
        let value = frequencies.entry(term).or_insert(0);
        *value = value.saturating_add(frequency.saturating_mul(3));
    }

    let mut local: Vec<(String, u32, f64)> = frequencies
        .into_iter()
        .map(|(text, frequency)| {
            let local_weight = 1.0 + (frequency as f64).ln();
            (text, frequency, local_weight)
        })
        .collect();
    local.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    local.truncate(
        max_query_terms
            .saturating_mul(IDF_PREFILTER_MULT)
            .max(IDF_PREFILTER_FLOOR)
            .max(max_query_terms),
    );

    let documents = searcher.num_docs() as f64;
    let mut terms = Vec::with_capacity(local.len());
    for (text, frequency, local_weight) in local {
        let document_frequency = term_document_frequency(searcher, index, &text)? as f64;
        let idf = idf(document_frequency, documents);
        if idf.is_finite() && idf > 0.0 {
            terms.push(QueryTerm {
                text: text.clone(),
                label: labels.get(&text).cloned().unwrap_or(text),
                idf,
                frequency,
                weight: local_weight * idf,
            });
        }
    }
    terms.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.text.cmp(&right.text))
    });
    terms.truncate(max_query_terms);
    Ok(terms)
}

pub fn idf(document_frequency: f64, documents: f64) -> f64 {
    (1.0 + (documents - document_frequency + 0.5) / (document_frequency + 0.5)).ln()
}

pub fn candidate_query(
    index: &SearchIndex,
    terms: &[QueryTerm],
    params: &Bm25fParams,
) -> BooleanQuery {
    let maximum_weight = terms.first().map_or(1.0, |term| term.weight);
    let mut clauses = Vec::<(Occur, Box<dyn Query>)>::with_capacity(terms.len() * 2);
    for term in terms {
        let weight = (term.weight / maximum_weight).max(0.1) as f32;
        clauses.push((
            Occur::Should,
            Box::new(BoostQuery::new(
                Box::new(TermQuery::new(
                    Term::from_field_text(index.fields.body, &term.text),
                    IndexRecordOption::WithFreqs,
                )),
                weight,
            )),
        ));
        clauses.push((
            Occur::Should,
            Box::new(BoostQuery::new(
                Box::new(TermQuery::new(
                    Term::from_field_text(index.fields.title, &term.text),
                    IndexRecordOption::WithFreqs,
                )),
                weight * params.title_weight,
            )),
        ));
    }
    BooleanQuery::new(clauses)
}

fn term_document_frequency(
    searcher: &Searcher,
    index: &SearchIndex,
    text: &str,
) -> Result<u64, SearchError> {
    let body = searcher.doc_freq(&Term::from_field_text(index.fields.body, text))?;
    let title = searcher.doc_freq(&Term::from_field_text(index.fields.title, text))?;
    Ok(body.saturating_add(title).min(searcher.num_docs()))
}
