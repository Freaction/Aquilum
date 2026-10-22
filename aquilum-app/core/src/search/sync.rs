use super::error::SearchError;
use super::index::SearchIndex;
use super::index_document::{apply, probe, probe_one, remove_note};
use super::metadata::{self, FileState, PathKey};
use super::pages;
use super::paths::{is_hidden, is_markdown, markdown_files};
use super::progress::IndexProgress;
use rusqlite::{Connection, Transaction};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tantivy::IndexWriter;

pub const INDEX_MEMORY_BUDGET: usize = 40_000_000;
const INDEX_THREADS: usize = 2;
const COMMIT_BATCH_SIZE: usize = 10_000;

pub struct IndexSynchronizer {
    output: Output,
    connection: Connection,
}

struct Output {
    root: PathBuf,
    index: Arc<SearchIndex>,
    writer: Option<IndexWriter>,
    progress: Arc<IndexProgress>,
    pending: bool,
}

impl IndexSynchronizer {
    pub fn new(root: PathBuf, index: Arc<SearchIndex>, metadata_path: &Path, progress: Arc<IndexProgress>) -> Result<Self, SearchError> {
        Ok(Self { output: Output { root, index, writer: None, progress, pending: false }, connection: metadata::open(metadata_path)? })
    }

    pub fn full_scan(&mut self, cancelled: &AtomicBool) -> Result<(), SearchError> {
        let output = &mut self.output;
        output.progress.begin_scan();
        let mut known = metadata::load(&self.connection)?;
        let force = pages::count(&self.connection)? != output.index.document_count();
        let mut transaction = self.connection.transaction()?;
        let mut processed = 0;
        for path in markdown_files(&output.root) {
            if cancelled.load(Ordering::Relaxed) {
                return Ok(());
            }
            output.progress.scanned_one();
            processed += 1;
            if let Some(probe) = probe(&path, &mut known, force) {
                let writer = ensure_writer(&mut output.writer, &output.index)?;
                output.pending |= apply(&output.root, &output.index, writer, &transaction, &path, probe)?;
            }
            if processed >= COMMIT_BATCH_SIZE {
                output.commit()?;
                transaction.commit()?;
                transaction = self.connection.transaction()?;
                processed = 0;
            }
        }
        output.commit()?;
        output.remove_stale(&transaction, &known)?;
        transaction.commit()?;
        metadata::maintain(&self.connection)?;
        output.progress.completed(output.index.document_count());
        self.release_writer()
    }

    pub fn holds_writer(&self) -> bool {
        self.output.writer.is_some()
    }

    pub fn release_writer(&mut self) -> Result<(), SearchError> {
        if let Some(writer) = self.output.writer.take() {
            writer.wait_merging_threads()?;
        }
        Ok(())
    }

    pub fn apply_paths(&mut self, paths: HashSet<PathBuf>) -> Result<bool, SearchError> {
        let output = &mut self.output;
        let transaction = self.connection.transaction()?;
        let root = output.root.clone();
        for path in paths.into_iter().filter(|path| path.starts_with(&root) && !is_hidden(&root, path)) {
            match path.metadata() {
                Ok(found) if found.is_dir() => {
                    for note in markdown_files(&path) {
                        if let Ok(found) = note.metadata() {
                            output.update(&transaction, &note, &found)?;
                        }
                    }
                }
                Ok(found) if is_markdown(&path) => output.update(&transaction, &path, &found)?,
                Ok(_) => {}
                Err(_) => output.forget(&transaction, &path)?,
            }
        }
        let changed = output.pending;
        output.commit()?;
        transaction.commit()?;
        if changed {
            metadata::maintain(&self.connection)?;
        }
        self.release_writer()?;
        Ok(changed)
    }
}

impl Output {
    fn update(&mut self, transaction: &Transaction<'_>, path: &Path, found: &std::fs::Metadata) -> Result<(), SearchError> {
        let Some(probe) = probe_one(transaction, path, found)? else { return Ok(()) };
        let writer = ensure_writer(&mut self.writer, &self.index)?;
        self.pending |= apply(&self.root, &self.index, writer, transaction, path, probe)?;
        Ok(())
    }

    fn forget(&mut self, transaction: &Transaction<'_>, path: &Path) -> Result<(), SearchError> {
        for gone in metadata::paths_under(transaction, &path.to_string_lossy())? {
            remove_note(&self.index, ensure_writer(&mut self.writer, &self.index)?, transaction, &gone)?;
            self.pending = true;
        }
        Ok(())
    }

    fn remove_stale(&mut self, transaction: &Transaction<'_>, known: &HashMap<PathKey, FileState>) -> Result<(), SearchError> {
        let mut cursor = String::new();
        loop {
            let paths = metadata::paths_after(transaction, &cursor, COMMIT_BATCH_SIZE)?;
            let Some(last) = paths.last() else { return Ok(()) };
            cursor = last.clone();
            for path in paths.iter().filter(|path| known.contains_key(&metadata::key(path))) {
                remove_note(&self.index, ensure_writer(&mut self.writer, &self.index)?, transaction, path)?;
                self.pending = true;
            }
            self.commit()?;
        }
    }

    fn commit(&mut self) -> Result<(), SearchError> {
        if !std::mem::take(&mut self.pending) {
            return Ok(());
        }
        if let Some(writer) = self.writer.as_mut() {
            writer.commit()?;
            self.progress.committed(self.index.reload()?);
        }
        Ok(())
    }
}

fn ensure_writer<'w>(slot: &'w mut Option<IndexWriter>, index: &SearchIndex) -> Result<&'w mut IndexWriter, SearchError> {
    if slot.is_none() {
        *slot = Some(index.index.writer_with_num_threads(INDEX_THREADS, INDEX_MEMORY_BUDGET)?);
    }
    slot.as_mut().ok_or_else(|| SearchError::task("index writer is missing"))
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
