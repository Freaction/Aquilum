use super::terms::QueryTerm;
use crate::search::analysis::MAX_REASONS;
use crate::search::error::SearchError;
use crate::search::index::FieldStats;
use crate::settings::models::Bm25fParams;
use tantivy::postings::{Postings, SegmentPostings};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{DocId, DocSet, SegmentReader, Term};

const BODY_WEIGHT: f64 = 1.0;

pub struct SegmentTerms {
    title: Vec<Option<SegmentPostings>>,
    body: Vec<Option<SegmentPostings>>,
}

pub struct Candidate {
    pub id: DocId,
    pub title_length: u32,
    pub body_length: u32,
}

impl SegmentTerms {
    pub fn open(
        reader: &SegmentReader,
        title: Field,
        body: Field,
        terms: &[QueryTerm],
    ) -> Result<Self, SearchError> {
        Ok(Self {
            title: term_postings(reader, title, terms)?,
            body: term_postings(reader, body, terms)?,
        })
    }

    pub fn score(
        &mut self,
        terms: &[QueryTerm],
        candidate: &Candidate,
        stats: &FieldStats,
        params: &Bm25fParams,
    ) -> (f64, Vec<String>) {
        let mut score = 0.0;
        let mut reasons = Vec::new();
        for (index, term) in terms.iter().enumerate() {
            let title_tf = seek_term_freq(&mut self.title[index], candidate.id) as f64;
            let body_tf = seek_term_freq(&mut self.body[index], candidate.id) as f64;
            if (title_tf > 0.0 || body_tf > 0.0) && reasons.len() < MAX_REASONS {
                reasons.push(term.label.clone());
            }
            let title_norm = normalized_tf(
                title_tf,
                candidate.title_length,
                stats.average_title_length,
                params.b_title as f64,
            );
            let body_norm = normalized_tf(
                body_tf,
                candidate.body_length,
                stats.average_body_length,
                params.b_body as f64,
            );
            let field_tf = (params.title_weight as f64) * title_norm + BODY_WEIGHT * body_norm;
            if field_tf == 0.0 {
                continue;
            }
            let query_tf = term.frequency as f64;
            let query_factor = (params.k3 as f64 + 1.0) * query_tf / (params.k3 as f64 + query_tf);
            score += term.idf
                * ((params.k1 as f64 + 1.0) * field_tf / (params.k1 as f64 + field_tf))
                * query_factor;
        }
        (score, reasons)
    }
}

fn term_postings(
    reader: &SegmentReader,
    field: Field,
    terms: &[QueryTerm],
) -> Result<Vec<Option<SegmentPostings>>, SearchError> {
    let inverted = reader.inverted_index(field)?;
    let mut postings = Vec::with_capacity(terms.len());
    for term in terms {
        let tantivy_term = Term::from_field_text(field, &term.text);
        postings.push(inverted.read_postings(&tantivy_term, IndexRecordOption::WithFreqs)?);
    }
    Ok(postings)
}

pub fn normalized_tf(tf: f64, length: u32, average_length: f64, b: f64) -> f64 {
    if tf == 0.0 {
        return 0.0;
    }
    tf / (1.0 - b + b * length as f64 / average_length.max(1.0))
}

fn seek_term_freq(postings: &mut Option<SegmentPostings>, doc_id: DocId) -> u32 {
    let Some(postings) = postings.as_mut() else {
        return 0;
    };
    let current = postings.doc();
    if current > doc_id {
        return 0;
    }
    let at = if current < doc_id {
        postings.seek(doc_id)
    } else {
        current
    };
    if at == doc_id {
        postings.term_freq()
    } else {
        0
    }
}
