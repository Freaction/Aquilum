mod dirty;
mod read;
mod save;

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use aquilum_core::Core;
use aquilum_core::files::error::FileCommandError;

use crate::i18n::{t, t_with};
use dirty::Dirty;
pub use read::read;
use read::Stamp;
use save::Outcome;

const POLL: Duration = Duration::from_millis(20);
const RETRY: Duration = Duration::from_secs(5);

struct Disk {
    hash: String,
    stamp: Option<Stamp>,
    written: u64,
}

impl Disk {
    fn adopt(&mut self, loaded: read::Loaded) -> crop::Rope {
        (self.hash, self.stamp) = (loaded.hash, loaded.stamp);
        loaded.text
    }
}

fn lock(disk: &Mutex<Disk>) -> MutexGuard<'_, Disk> {
    disk.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct Note {
    pub path: PathBuf,
    disk: Arc<Mutex<Disk>>,
    pending: Option<Instant>,
    pub save_error: Option<String>,
    replaced: Option<crop::Rope>,
    queued: usize,
    done: (Sender<Outcome>, Receiver<Outcome>),
    seq: u64,
    dirty: Dirty,
}

impl Note {
    pub fn open(path: PathBuf) -> Result<(Note, crop::Rope), FileCommandError> {
        let loaded = read(&path)?;
        let disk = Arc::new(Mutex::new(Disk { hash: loaded.hash, stamp: loaded.stamp, written: 0 }));
        let dirty = Dirty::clean(loaded.text.byte_len());
        let note = Note { path, disk, pending: None, save_error: None, replaced: None, queued: 0, done: channel(), seq: 0, dirty };
        Ok((note, loaded.text))
    }

    pub fn edit(&mut self, debounce: Duration) {
        self.pending = Some(Instant::now() + debounce);
    }

    pub fn touch(&mut self, range: Range<usize>, inserted: usize) {
        self.dirty.edit(range, inserted);
    }

    pub fn hash(&self) -> String {
        lock(&self.disk).hash.clone()
    }

    fn settle(&mut self, len: usize) {
        self.dirty = Dirty::clean(len);
        lock(&self.disk).written = self.seq;
    }

    pub fn deadline(&self) -> Option<Instant> {
        if self.queued > 0 { Some(Instant::now() + POLL) } else { self.pending }
    }

    fn queue(&mut self, core: &Arc<Core>, text: &crop::Rope) {
        self.pending = None;
        self.queued += 1;
        let job = save::Job { after: self.seq, seq: self.seq + 1, change: self.dirty.splice() };
        self.seq += 1;
        self.dirty = Dirty::clean(text.byte_len());
        save::queue(Arc::clone(core), self.path.clone(), text.clone(), job, Arc::clone(&self.disk), self.done.0.clone());
    }

    pub fn tick(&mut self, core: &Arc<Core>, text: &crop::Rope, now: Instant) -> bool {
        let before = self.save_error.clone();
        self.collect();
        if self.pending.is_some_and(|due| due <= now) {
            self.queue(core, text);
        }
        self.save_error != before
    }

    fn collect(&mut self) {
        while let Ok(outcome) = self.done.1.try_recv() {
            self.queued -= 1;
            match outcome {
                Outcome::Saved => self.save_error = None,
                Outcome::Copied { copy, disk } => {
                    self.replaced = disk;
                    self.save_error = Some(t_with("editor.conflictCopy", &[("name", &copy)]));
                }
                Outcome::Unsaved => self.fail(t("editor.conflictUnsaved")),
                Outcome::Failed(reason) => self.fail(t_with("editor.saveFailed", &[("reason", &reason)])),
            }
        }
    }

    fn fail(&mut self, message: String) {
        self.pending = Some(Instant::now() + RETRY);
        self.save_error = Some(message);
    }

    pub fn commit(&mut self, core: &Arc<Core>, text: &crop::Rope) {
        if self.pending.is_some() {
            self.queue(core, text);
        }
    }

    pub fn flush(&mut self, core: &Arc<Core>, text: &crop::Rope) {
        self.commit(core, text);
        crate::disk::wait();
        self.collect();
    }

    pub fn take_replaced(&mut self) -> Option<crop::Rope> {
        let text = self.replaced.take()?;
        self.settle(text.byte_len());
        Some(text)
    }

    pub fn reload(&mut self) -> Option<crop::Rope> {
        if self.pending.is_some() || self.queued > 0 {
            return None;
        }
        let stamp = read::stamp(&self.path);
        if stamp.is_some() && stamp == lock(&self.disk).stamp {
            return None;
        }
        let hash = aquilum_core::files::document::hash_file(&self.path).ok()?;
        if hash == self.hash() {
            lock(&self.disk).stamp = stamp;
            return None;
        }
        let text = lock(&self.disk).adopt(read(&self.path).ok()?);
        self.settle(text.byte_len());
        Some(text)
    }

    pub fn is(&self, path: &Path) -> bool {
        aquilum_core::search::paths::same_path(&self.path, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_ignores_own_write_and_sees_external_one() {
        let dir = crate::test_dir::tempdir();
        let path = dir.path().join("a.md");
        std::fs::write(&path, "раз").unwrap();
        let (mut note, text) = Note::open(path.clone()).unwrap();
        assert_eq!(text, "раз");
        assert_eq!(note.reload(), None);
        std::fs::write(&path, "другой текст").unwrap();
        assert_eq!(note.reload().map(|text| text.to_string()).as_deref(), Some("другой текст"));
        note.edit(Duration::ZERO);
        assert!(note.deadline().is_some());
        assert_eq!(note.reload(), None, "несохранённая правка не затирается");
    }
}
