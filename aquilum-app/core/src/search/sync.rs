use super::error::SearchError;
use super::index::SearchIndex;
use super::index_document::{apply, apply_path, probe, remove_note};
use super::metadata::{self, FileState, PathKey};
use super::pages;
use super::paths::{is_hidden, is_markdown, is_visible_entry};
use super::progress::IndexProgress;
use rusqlite::{Connection, Transaction};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tantivy::IndexWriter;
use walkdir::WalkDir;

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
        for entry in WalkDir::new(&output.root).follow_links(false).into_iter().filter_entry(is_visible_entry).filter_map(Result::ok) {
            if cancelled.load(Ordering::Relaxed) {
                return Ok(());
            }
            if !entry.file_type().is_file() || !is_markdown(entry.path()) {
                continue;
            }
            output.progress.scanned_one();
            processed += 1;
            if let Some(probe) = probe(entry.path(), &mut known, force) {
                let writer = ensure_writer(&mut output.writer, &output.index)?;
                output.pending |= apply(&output.root, &output.index, writer, &transaction, entry.path(), probe)?;
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
        Ok(())
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
        for path in paths {
            if path.starts_with(&output.root) && is_markdown(&path) && !is_hidden(&output.root, &path) {
                let writer = ensure_writer(&mut output.writer, &output.index)?;
                output.pending |= apply_path(&output.root, &output.index, writer, &transaction, &path)?;
            }
        }
        let changed = output.pending;
        output.commit()?;
        transaction.commit()?;
        if changed {
            metadata::maintain(&self.connection)?;
        }
        Ok(changed)
    }
}

impl Output {
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
