use super::{remove_note, upsert, Probe};
use crate::search::created;
use crate::search::error::SearchError;
use crate::search::fields;
use crate::search::index::SearchIndex;
use crate::search::pages::{self, Page, Pages};
use crate::search::paths::relative_slash_path;
use crate::search::tasks;
use crate::search::wiki;
use rusqlite::Transaction;
use std::collections::HashMap;
use std::path::Path;
use tantivy::{IndexWriter, TantivyDocument, Term};

pub fn apply(root: &Path, index: &SearchIndex, writer: &IndexWriter, transaction: &Transaction<'_>, path: &Path, probe: Probe) -> Result<bool, SearchError> {
    let key = path.to_string_lossy();
    match index_note(root, index, writer, transaction, path, &key, probe) {
        Err(SearchError::Io { .. }) => {
            remove_note(index, writer, transaction, &key)?;
            Ok(true)
        }
        indexed => indexed,
    }
}

fn index_note(root: &Path, index: &SearchIndex, writer: &IndexWriter, transaction: &Transaction<'_>, path: &Path, key: &str, probe: Probe) -> Result<bool, SearchError> {
    let mut pages = Pages::open(path)?;
    let mut old = match probe.up_to_date {
        true => pages::stored(transaction, key)?,
        false => {
            writer.delete_term(Term::from_field_text(index.fields.id, key));
            pages::remove(transaction, key)?;
            HashMap::new()
        }
    };
    wiki::register(transaction, root, path)?;
    let (mut repeats, mut ordinal, mut changed, mut created) = (HashMap::new(), 0, false, probe.created);
    while let Some(page) = pages.next()? {
        if page.first {
            created = created::declared(&page.text).unwrap_or(probe.created);
        }
        let repeat = repeats.entry(page.hash).or_insert(0usize);
        let page_key = pages::key(key, &page, *repeat);
        *repeat += 1;
        match old.remove(&page_key) {
            Some(id) => pages::place(transaction, id, ordinal, &page)?,
            None => {
                store_page(transaction, path, &page_key, ordinal, &page)?;
                add_page(root, index, writer, path, &page, &page_key)?;
                changed = true;
            }
        }
        ordinal += 1;
    }
    for (gone, id) in old {
        writer.delete_term(Term::from_field_text(index.fields.page, &gone));
        pages::delete(transaction, id)?;
        changed = true;
    }
    upsert(transaction, key, probe.fingerprint, created)?;
    Ok(changed)
}

pub(super) fn store_page(transaction: &Transaction<'_>, path: &Path, page_key: &str, ordinal: usize, page: &Page) -> Result<(), SearchError> {
    let id = pages::insert(transaction, &path.to_string_lossy(), page_key, ordinal, page)?;
    wiki::insert_links(transaction, id, path, &page.text)?;
    tasks::insert(transaction, id, path, &tasks::note_tasks(&page.text))?;
    fields::store(transaction, id, path, &fields::page_fields(&page.text, page.first))
}

fn add_page(root: &Path, index: &SearchIndex, writer: &IndexWriter, path: &Path, page: &Page, key: &str) -> Result<(), SearchError> {
    let mut document = TantivyDocument::new();
    document.add_text(index.fields.id, path.to_string_lossy());
    document.add_text(index.fields.page, key);
    if page.first {
        document.add_text(index.fields.path, relative_slash_path(root, path));
        document.add_text(index.fields.title, crate::search::paths::title(path));
    }
    document.add_text(index.fields.body, &page.text);
    writer.add_document(document)?;
    Ok(())
}
