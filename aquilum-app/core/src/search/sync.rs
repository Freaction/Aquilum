use super::error::SearchError;
use super::fields;
use super::tasks;
use super::index::SearchIndex;
use super::index_document::{apply_path, apply_prepared, prepare_file, probe, PreparedFile, Probe};
use super::metadata::{self, FileState, PathKey};
use super::paths::{is_hidden, is_markdown, is_visible_entry};
use super::progress::IndexProgress;
use super::wiki;
use rusqlite::{params, Connection, Transaction};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tantivy::{IndexWriter, Term};
use walkdir::WalkDir;

pub const INDEX_MEMORY_BUDGET: usize = 40_000_000;
const INDEX_THREADS: usize = 2;
const COMMIT_BATCH_SIZE: usize = 10_000;
const PREPARE_BATCH_SIZE: usize = 64;

pub struct IndexSynchronizer {
    root: PathBuf,
    index: Arc<SearchIndex>,
    writer: Option<IndexWriter>,
    connection: Connection,
    progress: Arc<IndexProgress>,
}

struct PendingPrepare {
    key: String,
    probe: Probe,
    path: PathBuf,
}

impl IndexSynchronizer {
    pub fn new(
        root: PathBuf,
        index: Arc<SearchIndex>,
        metadata_path: &Path,
        progress: Arc<IndexProgress>,
    ) -> Result<Self, SearchError> {
        Ok(Self {
            root,
            index,
            writer: None,
            connection: metadata::open(metadata_path)?,
            progress,
        })
    }

    pub fn full_scan(&mut self, cancelled: &AtomicBool) -> Result<(), SearchError> {
        self.progress.begin_scan();
        let mut known = metadata::load(&self.connection)?;
        let force_reindex = known.len() as u64 != self.index.document_count();
        let mut transaction = self.connection.transaction()?;
        let mut pending = 0;
        let mut processed = 0;
        let mut prepare_batch = Vec::<PendingPrepare>::with_capacity(PREPARE_BATCH_SIZE);

        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(is_visible_entry)
            .filter_map(Result::ok)
        {
            if cancelled.load(Ordering::Relaxed) {
                return Ok(());
            }
            if !entry.file_type().is_file() || !is_markdown(entry.path()) {
                continue;
            }
            self.progress.scanned_one();
            processed += 1;
            if let Some((key, file_probe)) = probe(entry.path(), &mut known, force_reindex) {
                prepare_batch.push(PendingPrepare {
                    key,
                    probe: file_probe,
                    path: entry.path().to_path_buf(),
                });
                if prepare_batch.len() >= PREPARE_BATCH_SIZE {
                    flush_prepare_batch(
                        &self.root,
                        &self.index,
                        &mut self.writer,
                        &transaction,
                        &mut prepare_batch,
                        &mut pending,
                    )?;
                }
            }
            if processed >= COMMIT_BATCH_SIZE {
                flush_prepare_batch(
                    &self.root,
                    &self.index,
                    &mut self.writer,
                    &transaction,
                    &mut prepare_batch,
                    &mut pending,
                )?;
                if pending > 0 {
                    commit(&mut self.writer, &self.index, &self.progress)?;
                }
                transaction.commit()?;
                transaction = self.connection.transaction()?;
                pending = 0;
                processed = 0;
            }
        }
        flush_prepare_batch(
            &self.root,
            &self.index,
            &mut self.writer,
            &transaction,
            &mut prepare_batch,
            &mut pending,
        )?;
        if pending > 0 {
            commit(&mut self.writer, &self.index, &self.progress)?;
        }
        remove_stale(
            &self.index,
            &mut self.writer,
            &transaction,
            &known,
            &self.progress,
        )?;
        transaction.commit()?;
        metadata::maintain(&self.connection)?;
        self.progress.completed(self.index.document_count());
        Ok(())
    }

    pub fn holds_writer(&self) -> bool {
        self.writer.is_some()
    }

    pub fn release_writer(&mut self) -> Result<(), SearchError> {
        if let Some(writer) = self.writer.take() {
            writer.wait_merging_threads()?;
        }
        Ok(())
    }

    pub fn apply_paths(&mut self, paths: HashSet<PathBuf>) -> Result<bool, SearchError> {
        let transaction = self.connection.transaction()?;
        let mut changed = false;
        for path in paths {
            if !path.starts_with(&self.root) || !is_markdown(&path) || is_hidden(&self.root, &path)
            {
                continue;
            }
            let writer = ensure_writer(&mut self.writer, &self.index)?;
            changed |= apply_path(&self.root, &self.index, writer, &transaction, &path)?;
        }
        if changed {
            commit(&mut self.writer, &self.index, &self.progress)?;
        }
        transaction.commit()?;
        if changed {
            metadata::maintain(&self.connection)?;
        }
        Ok(changed)
    }
}

fn ensure_writer<'w>(
    slot: &'w mut Option<IndexWriter>,
    index: &SearchIndex,
) -> Result<&'w mut IndexWriter, SearchError> {
    if slot.is_none() {
        *slot = Some(index.index.writer_with_num_threads(INDEX_THREADS, INDEX_MEMORY_BUDGET)?);
    }
    slot.as_mut().ok_or_else(|| SearchError::task("index writer is missing"))
}

fn flush_prepare_batch(
    root: &Path,
    index: &SearchIndex,
    writer: &mut Option<IndexWriter>,
    transaction: &Transaction<'_>,
    batch: &mut Vec<PendingPrepare>,
    pending: &mut usize,
) -> Result<(), SearchError> {
    if batch.is_empty() {
        return Ok(());
    }
    let writer = ensure_writer(writer, index)?;
    for (key, probe, file) in prepare_parallel(std::mem::take(batch)) {
        let Some(file) = file else {
            continue;
        };
        let changed = apply_prepared(root, index, writer, transaction, &key, probe, file)? as usize;
        *pending += changed;
    }
    Ok(())
}

fn prepare_parallel(batch: Vec<PendingPrepare>) -> Vec<(String, Probe, Option<PreparedFile>)> {
    let workers = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4)
        .clamp(1, 8)
        .min(batch.len().max(1));
    if batch.len() < 2 || workers == 1 {
        return batch.into_iter().map(prepare_one).collect();
    }

    let chunk_len = batch.len().div_ceil(workers);
    let mut remaining = batch;
    let mut chunks = Vec::with_capacity(workers);
    while !remaining.is_empty() {
        let take = chunk_len.min(remaining.len());
        chunks.push(remaining.drain(..take).collect::<Vec<_>>());
    }

    thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| scope.spawn(move || chunk.into_iter().map(prepare_one).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_default())
            .collect()
    })
}

fn prepare_one(item: PendingPrepare) -> (String, Probe, Option<PreparedFile>) {
    let PendingPrepare { key, probe, path } = item;
    let prepared = prepare_file(path, probe.fingerprint);
    (key, probe, prepared)
}

fn remove_stale(
    index: &SearchIndex,
    writer: &mut Option<IndexWriter>,
    transaction: &Transaction<'_>,
    known: &HashMap<PathKey, FileState>,
    progress: &IndexProgress,
) -> Result<(), SearchError> {
    let mut cursor = String::new();
    loop {
        let paths = metadata::paths_after(transaction, &cursor, COMMIT_BATCH_SIZE)?;
        let Some(last) = paths.last() else { break };
        cursor = last.clone();
        let mut removed = 0;
        for path in paths {
            if !known.contains_key(&metadata::key(&path)) {
                continue;
            }
            ensure_writer(writer, index)?.delete_term(Term::from_field_text(index.fields.id, &path));
            wiki::remove_document(transaction, Path::new(&path))?;
            fields::remove_document(transaction, Path::new(&path))?;
            tasks::remove_document(transaction, Path::new(&path))?;
            transaction.execute("DELETE FROM documents WHERE path=?1", params![path])?;
            removed += 1;
        }
        if removed > 0 {
            commit(writer, index, progress)?;
        }
    }
    Ok(())
}

fn commit(
    writer: &mut Option<IndexWriter>,
    index: &SearchIndex,
    progress: &IndexProgress,
) -> Result<(), SearchError> {
    let Some(writer) = writer.as_mut() else {
        return Ok(());
    };
    writer.commit()?;
    progress.committed(index.reload()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::IndexSynchronizer;
    use crate::search::index::SearchIndex;
    use crate::search::progress::IndexProgress;
    use std::collections::HashSet;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn the_writer_lives_only_while_there_is_something_to_write() {
        let directory = tempfile::tempdir().unwrap();
        let vault = crate::search::paths::canonical_path(directory.path()).join("vault");
        fs::create_dir_all(&vault).unwrap();
        let note = vault.join("Заметка.md");
        fs::write(&note, "первое слово").unwrap();
        let index = Arc::new(SearchIndex::open(&directory.path().join("index")).unwrap());
        let mut synchronizer = IndexSynchronizer::new(
            vault.clone(),
            Arc::clone(&index),
            &directory.path().join("documents.sqlite3"),
            Arc::new(IndexProgress::new(0, 1)),
        )
        .unwrap();
        let cancelled = AtomicBool::new(false);
        assert!(!synchronizer.holds_writer(), "открытие не создаёт писателя");

        synchronizer.full_scan(&cancelled).unwrap();
        assert!(synchronizer.holds_writer(), "новая заметка записана писателем");
        synchronizer.release_writer().unwrap();
        assert!(!synchronizer.holds_writer());
        drop(index.index.writer::<tantivy::TantivyDocument>(15_000_000).expect("блокировка индекса отпущена"));

        synchronizer.full_scan(&cancelled).unwrap();
        assert!(!synchronizer.holds_writer(), "неизменённая база писателя не создаёт");

        fs::write(&note, "второе слово").unwrap();
        synchronizer.apply_paths(HashSet::from([note])).unwrap();
        assert!(synchronizer.holds_writer());
        assert!(!index.search(&crate::search::query::Request::new("второе")).unwrap().results.is_empty(), "правка видна поиску сразу");
    }
}
