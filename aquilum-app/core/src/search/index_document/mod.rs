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
pub(crate) use note::add_page;

#[cfg(test)]
pub fn index_text(transaction: &Transaction<'_>, root: &Path, path: &Path, body: &str) -> Result<(), SearchError> {
    let key = path.to_string_lossy();
    upsert(transaction, root, path, (0, 0), CreatedAt { nanos: 0, source: CreatedSource::ModifiedTime })?;
    pages::remove(transaction, &key)?;
    let Some(page) = pages::Pages::of(&body).next()? else { return Ok(()) };
    note::store_page(transaction, path, &pages::Keys::default().next(&key, &page), 0, &page)
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

pub fn probe_one(transaction: &Transaction<'_>, path: &Path, found: &std::fs::Metadata) -> Result<Option<Probe>, SearchError> {
    let previous = metadata::current(transaction, &path.to_string_lossy())?;
    Ok(classify(previous, metadata::fingerprint(found), created::from_file_metadata(found), false))
}

pub fn remove_note(index: &SearchIndex, writer: &IndexWriter, transaction: &Transaction<'_>, key: &str) -> Result<(), SearchError> {
    writer.delete_term(Term::from_field_text(index.fields.id, key));
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

fn upsert(transaction: &Transaction<'_>, root: &Path, path: &Path, fingerprint: Fingerprint, created: CreatedAt) -> Result<(), SearchError> {
    let sql = "INSERT OR REPLACE INTO documents(path, relative_key, title_key, modified_ns, size, analyzer_version, created_ns, created_source)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
    transaction.prepare_cached(sql)?.execute(params![
        path.to_string_lossy(),
        wiki::target::relative_key(root, path),
        wiki::target::title_key(path),
        fingerprint.0,
        fingerprint.1,
        ANALYZER_VERSION,
        created.nanos,
        created.source.stored()
    ])?;
    Ok(())
}
