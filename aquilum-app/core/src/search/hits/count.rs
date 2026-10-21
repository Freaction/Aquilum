use crate::search::error::SearchError;
use crate::search::index::SearchFields;
use crate::search::pages;
use std::collections::HashSet;
use tantivy::postings::{Postings, SegmentPostings};
use tantivy::schema::IndexRecordOption;
use tantivy::{DocId, DocSet, InvertedIndexReader, Searcher, SegmentReader, TERMINATED};

pub struct Hits {
    pub counts: Vec<u32>,
    pub pages: HashSet<String>,
}

pub fn note_hits(searcher: &Searcher, fields: SearchFields, path: &str, terms: &[String]) -> Result<Hits, SearchError> {
    let mut hits = Hits { counts: vec![0u32; terms.len()], pages: HashSet::new() };
    for segment in searcher.segment_readers() {
        let documents = note_pages(segment, fields, path)?;
        if documents.is_empty() {
            continue;
        }
        let inverted = segment.inverted_index(fields.body)?;
        for (term, prefix) in terms.iter().enumerate() {
            each_prefixed(&inverted, prefix, IndexRecordOption::WithFreqs, |_, mut postings| {
                for (document, key) in &documents {
                    if postings.doc() == TERMINATED {
                        break;
                    }
                    if postings.doc() <= *document && postings.seek(*document) == *document {
                        hits.counts[term] += postings.term_freq();
                        hits.pages.insert(key.clone());
                    }
                }
            })?;
        }
    }
    Ok(hits)
}

fn note_pages(segment: &SegmentReader, fields: SearchFields, path: &str) -> Result<Vec<(DocId, String)>, SearchError> {
    let mut documents = Vec::new();
    each_prefixed(&*segment.inverted_index(fields.page)?, &pages::prefix(path), IndexRecordOption::Basic, |key, mut postings| {
        while postings.doc() != TERMINATED {
            if segment.alive_bitset().is_none_or(|alive| alive.is_alive(postings.doc())) {
                documents.push((postings.doc(), String::from_utf8_lossy(key).into_owned()));
            }
            postings.advance();
        }
    })?;
    documents.sort_unstable_by_key(|(document, _)| *document);
    Ok(documents)
}

fn each_prefixed(inverted: &InvertedIndexReader, prefix: &str, option: IndexRecordOption, mut each: impl FnMut(&[u8], SegmentPostings)) -> Result<(), SearchError> {
    let mut stream = inverted.terms().range().ge(prefix.as_bytes()).into_stream()?;
    while stream.advance() && stream.key().starts_with(prefix.as_bytes()) {
        each(stream.key(), inverted.read_postings_from_terminfo(stream.value(), option)?);
    }
    Ok(())
}

pub fn total(terms: &[String], counts: &[u32]) -> usize {
    terms
        .iter()
        .zip(counts)
        .filter(|(term, _)| !terms.iter().any(|other| other != *term && term.starts_with(other.as_str())))
        .map(|(_, &count)| count as usize)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::total;

    #[test]
    fn a_term_inside_another_prefix_is_counted_once() {
        let terms = ["тест".to_owned(), "тестирован".to_owned(), "полн".to_owned()];
        assert_eq!(total(&terms, &[5, 2, 3]), 8);
    }
}
