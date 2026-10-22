use crate::search::index::SearchIndex;
use crate::search::progress::IndexProgress;
use crate::search::sync::IndexSynchronizer;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RUNS: u32 = 3;

pub fn best(mut run: impl FnMut()) -> Duration {
    (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            run();
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

pub fn ms(value: Duration) -> f64 {
    value.as_secs_f64() * 1000.0
}

pub fn write_note(vault: &Path, index: usize, paragraphs: usize) {
    let mut content = format!(
        "# Note {index}\n\nAquilum architecture indexing topic {}. This note contains realistic amounts of text to properly stress the BM25F algorithm and the filesystem tokenizer.\n\n",
        index % 97
    );
    for _ in 0..paragraphs {
        content.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. ");
    }
    content.push_str("\n\n## References\n");
    for offset in 1..=5 {
        let target = index.saturating_sub(offset);
        if target != index {
            content.push_str(&format!("- [[Note {target}]]\n"));
        }
    }
    fs::write(vault.join(format!("Note {index}.md")), content).unwrap();
}

pub fn seed_vault(vault: &Path, document_count: usize) {
    fs::create_dir_all(vault).unwrap();
    for index in 0..document_count {
        write_note(vault, index, 50);
    }
}

pub struct BenchIndex {
    pub vault: PathBuf,
    pub search_index: Arc<SearchIndex>,
    pub synchronizer: IndexSynchronizer,
    pub cancelled: AtomicBool,
    pub metadata: PathBuf,
    pub directory: tempfile::TempDir,
}

pub fn open_bench_index(document_count: usize) -> BenchIndex {
    let directory = tempfile::tempdir().unwrap();
    let vault = directory.path().join("vault");
    seed_vault(&vault, document_count);
    index_vault(directory, vault)
}

pub fn index_vault(directory: tempfile::TempDir, vault: PathBuf) -> BenchIndex {
    let search_index = Arc::new(SearchIndex::open(&directory.path().join("index")).unwrap());
    let metadata = directory.path().join("documents.sqlite3");
    let synchronizer = IndexSynchronizer::new(
        vault.clone(),
        Arc::clone(&search_index),
        &metadata,
        Arc::new(IndexProgress::new(0, 1)),
    )
    .unwrap();
    BenchIndex {
        directory,
        vault,
        search_index,
        synchronizer,
        cancelled: AtomicBool::new(false),
        metadata,
    }
}

pub fn directory_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

pub fn record(name: &str, line: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.artifacts/benchmarks");
    if fs::create_dir_all(&root).is_err() {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(format!("{name}.log")))
    {
        let _ = writeln!(file, "[{timestamp}] {line}");
    }
}
