use crate::app_core::{Core, CoreEvent, EventSink};
use crate::files::gate;
use crate::history::Source;
use crate::search::paths::{canonical_path, is_visible_entry};
use crate::settings::models::SyncSettings;
use crate::sync::scan::{relative, stamp, stat};
use crate::sync::{run, Large, Outcome, Progress, Records, Remote, Size, SyncError, Vault, Watch};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;

pub const NOTES_ONLY: SyncSettings = SyncSettings { enabled: true, trash: false, history: false, interval: 300 };
pub const EVERYTHING: SyncSettings = SyncSettings { enabled: true, trash: true, history: true, interval: 300 };

pub struct Device {
    directory: tempfile::TempDir,
    pub core: Arc<Core>,
    pub root: PathBuf,
    pub records: Records,
    pub scope: Cell<SyncSettings>,
    pub allow_deletions: Cell<bool>,
    pub allowed: RefCell<Vec<String>>,
    pub measured: RefCell<(Vec<Large>, Size)>,
    until: Cell<Option<usize>>,
    done: Cell<usize>,
}

impl Watch for Device {
    fn measured(&self, large: Vec<Large>, size: Size) {
        *self.measured.borrow_mut() = (large, size);
    }

    fn progress(&self, progress: Progress) {
        self.done.set(progress.done);
    }

    fn stopped(&self) -> bool {
        self.until.get().is_some_and(|until| self.done.get() >= until)
    }
}

impl Device {
    pub fn new() -> Self {
        Self::with(NOTES_ONLY)
    }

    pub fn with(scope: SyncSettings) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let (root, data) = (directory.path().join("vault"), directory.path().join("data"));
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&data).unwrap();
        let root = canonical_path(&root);
        let events: Arc<dyn EventSink> = Arc::new(|_: CoreEvent| {});
        let core = Core::open(&data, events);
        Self { records: Records::memory().unwrap(), core, root, directory, scope: Cell::new(scope), allow_deletions: Cell::new(false), allowed: RefCell::new(Vec::new()), measured: RefCell::default(), until: Cell::new(None), done: Cell::new(0) }
    }

    pub fn sync(&self, remote: &dyn Remote) -> Outcome {
        self.try_sync(remote).unwrap()
    }

    pub fn sync_until(&self, remote: &dyn Remote, done: usize) -> Outcome {
        self.until.set(Some(done));
        let outcome = self.sync(remote);
        self.until.set(None);
        outcome
    }

    pub fn try_sync(&self, remote: &dyn Remote) -> Result<Outcome, SyncError> {
        self.done.set(0);
        let staging = self.directory.path().join("staging");
        let vault = Vault {
            core: &self.core,
            root: &self.root,
            remote,
            records: &self.records,
            staging: &staging,
            offset_seconds: 0,
            scope: self.scope.get(),
            watch: self,
            allow_deletions: self.allow_deletions.get(),
            allowed: &self.allowed.borrow(),
        };
        run(&vault)
    }

    pub fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    pub fn keep_history(&self) {
        self.core.ui_state.resolve_workspace(&self.root.to_string_lossy(), 0).unwrap();
    }

    pub fn save(&self, path: &str, text: &str) {
        gate::write(&self.core, &self.root.join(path), text, None, Source::Me, None).unwrap();
    }

    pub fn versions(&self, note: &str) -> usize {
        let dir = self.root.join(".aquilum").join("history").join(note);
        fs::read_dir(dir).map_or(0, |entries| entries.filter_map(Result::ok).filter(|entry| !entry.path().extension().is_some_and(|ext| ext == "name")).count())
    }

    pub fn write_bytes(&self, path: &str, bytes: &[u8]) {
        fs::write(self.root.join(path), bytes).unwrap();
    }

    pub fn trash(&self, path: &str) {
        gate::trash(&self.core, &self.root, &self.root.join(path)).unwrap();
    }

    pub fn kept_text(&self) -> String {
        let keep = |entry: &walkdir::DirEntry| !entry.path().starts_with(self.root.join(".aquilum"));
        let files = WalkDir::new(&self.root).into_iter().filter_entry(keep).filter_map(Result::ok).filter(|entry| entry.file_type().is_file());
        files.filter_map(|entry| fs::read_to_string(entry.path()).ok()).collect::<Vec<_>>().join("\n")
    }

    pub fn copies(&self) -> Vec<String> {
        self.files().into_iter().filter(|path| path.contains("(конфликт")).collect()
    }

    pub fn read(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.root.join(path)).ok()
    }

    pub fn delete(&self, path: &str) {
        fs::remove_file(self.root.join(path)).unwrap();
    }

    pub fn set_modified(&self, path: &str, millis: u64) {
        stamp(&self.root.join(path), millis).unwrap();
    }

    pub fn modified(&self, path: &str) -> u64 {
        stat(&fs::metadata(self.root.join(path)).unwrap()).unwrap().modified
    }

    pub fn files(&self) -> Vec<String> {
        let mut files: Vec<String> = WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(is_visible_entry)
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| relative(&self.root, entry.path()))
            .collect();
        files.sort();
        files
    }

    pub fn trashed(&self) -> bool {
        WalkDir::new(self.root.join(".trash")).into_iter().filter_map(Result::ok).any(|entry| entry.file_type().is_file())
    }
}
