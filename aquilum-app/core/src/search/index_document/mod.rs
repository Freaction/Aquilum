mod note;
#[cfg(test)]
mod tests;

use super::created::{self, CreatedAt, CreatedSource};
use super::error::SearchError;
use super::index::SearchIndex;
use super::metadata::{self, FileState, Fingerprint, PathKey};
use super::pages;
use super::wiki;
use super::ANALYZER_VERSION;
use rusqlite::{params, Transaction};
use std::collections::HashMap;
use std::path::Path;
use tantivy::{IndexWriter, Term};

pub use note::apply;

#[cfg(test)]
pub fn index_text(transaction: &Transaction<'_>, root: &Path, path: &Path, body: &str) -> Result<(), SearchError> {
    let key = path.to_string_lossy();
    wiki::register(transaction, root, path)?;
    pages::remove(transaction, &key)?;
    let Some(page) = pages::Pages::of(&body).next()? else { return Ok(()) };
    note::store_page(transaction, path, &key, 0, &page)
}

#[derive(Clone, Copy)]
pub struct Probe {
    pub fingerprint: Fingerprint,
    pub up_to_date: bool,
    pub created: CreatedAt,
}

pub fn probe(path: &Path, known: &mut HashMap<PathKey, FileState>, force: bool) -> Option<Probe> {
    let file_metadata = path.metadata().ok()?;
    let previous = known.remove(&metadata::key(&path.to_string_lossy()));
    classify(previous, metadata::fingerprint(&file_metadata), created::from_file_metadata(&file_metadata), force)
}

pub fn apply_path(
    root: &Path,
    index: &SearchIndex,
    writer: &IndexWriter,
    transaction: &Transaction<'_>,
    path: &Path,
) -> Result<bool, SearchError> {
    let key = path.to_string_lossy();
    if !path.is_file() {
        remove_note(index, writer, transaction, &key)?;
        return Ok(true);
    }

    let file_metadata = path.metadata()?;
    let fingerprint = metadata::fingerprint(&file_metadata);
    let Some(probe) = classify(metadata::current(transaction, &key)?, fingerprint, created::from_file_metadata(&file_metadata), false) else {
        return Ok(false);
    };
    apply(root, index, writer, transaction, path, probe)
}

pub fn remove_note(index: &SearchIndex, writer: &IndexWriter, transaction: &Transaction<'_>, key: &str) -> Result<(), SearchError> {
    writer.delete_term(Term::from_field_text(index.fields.id, key));
    wiki::remove_document(transaction, Path::new(key))?;
    pages::remove(transaction, key)?;
    transaction.execute("DELETE FROM documents WHERE path=?1", params![key])?;
    Ok(())
}

fn classify(
    previous: Option<FileState>,
    fingerprint: Fingerprint,
    candidate: CreatedAt,
    force: bool,
) -> Option<Probe> {
    let up_to_date = previous
        .as_ref()
        .is_some_and(|value| value.analyzer_version == ANALYZER_VERSION);
    if !force && up_to_date && previous.as_ref().is_some_and(|value| value.fingerprint == fingerprint)
    {
        return None;
    }
    let carried = previous
        .as_ref()
        .map(|value| value.created)
        .filter(|value| value.source != CreatedSource::Frontmatter);
    let created = created::resolve(carried, candidate);
    Some(Probe {
        fingerprint,
        up_to_date: !force && up_to_date,
        created,
    })
}

fn upsert(transaction: &Transaction<'_>, path: &str, fingerprint: Fingerprint, created: CreatedAt) -> Result<(), SearchError> {
    transaction.execute(
        "INSERT INTO documents(path, modified_ns, size, analyzer_version, created_ns, created_source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(path) DO UPDATE SET modified_ns=excluded.modified_ns, size=excluded.size,
           analyzer_version=excluded.analyzer_version, created_ns=excluded.created_ns, created_source=excluded.created_source",
        params![path, fingerprint.0, fingerprint.1, ANALYZER_VERSION, created.nanos, created.source.stored()],
    )?;
    Ok(())
}
