use crate::search::error::SearchError;
use crate::search::index::SearchFields;
use crate::search::pages;
use tantivy::schema::IndexRecordOption;
use tantivy::{DocId, DocSet, SegmentReader, TERMINATED};

pub fn note_pages(segment: &SegmentReader, fields: SearchFields, path: &str, mut each: impl FnMut(DocId, &[u8])) -> Result<(), SearchError> {
    let (inverted, alive, prefix) = (segment.inverted_index(fields.page)?, segment.alive_bitset(), pages::prefix(path));
    let mut stream = inverted.terms().range().ge(prefix.as_bytes()).into_stream()?;
    while stream.advance() && stream.key().starts_with(prefix.as_bytes()) {
        let mut postings = inverted.read_postings_from_terminfo(stream.value(), IndexRecordOption::Basic)?;
        while postings.doc() != TERMINATED {
            if alive.is_none_or(|alive| alive.is_alive(postings.doc())) {
                each(postings.doc(), stream.key());
            }
            postings.advance();
        }
    }
    Ok(())
}
