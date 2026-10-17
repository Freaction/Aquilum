use crate::search::error::SearchError;
use std::collections::BTreeMap;
use tantivy::postings::Postings;
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{DocAddress, DocId, DocSet, Searcher, TERMINATED};

pub fn body_counts(
    searcher: &Searcher,
    field: Field,
    terms: &[String],
    addresses: &[DocAddress],
) -> Result<Vec<u32>, SearchError> {
    let mut counts = vec![0u32; terms.len()];
    let mut by_segment = BTreeMap::<u32, Vec<DocId>>::new();
    for address in addresses {
        by_segment.entry(address.segment_ord).or_default().push(address.doc_id);
    }
    for (segment, mut documents) in by_segment {
        documents.sort_unstable();
        let inverted = searcher.segment_reader(segment).inverted_index(field)?;
        for (term, prefix) in terms.iter().enumerate() {
            let mut stream = inverted.terms().range().ge(prefix.as_bytes()).into_stream()?;
            while stream.advance() && stream.key().starts_with(prefix.as_bytes()) {
                let mut postings = inverted.read_postings_from_terminfo(stream.value(), IndexRecordOption::WithFreqs)?;
                for &document in &documents {
                    if postings.doc() == TERMINATED {
                        break;
                    }
                    if postings.doc() <= document && postings.seek(document) == document {
                        counts[term] += postings.term_freq();
                    }
                }
            }
        }
    }
    Ok(counts)
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
