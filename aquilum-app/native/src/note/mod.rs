mod read;
mod save;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use aquilum_core::Core;
use aquilum_core::files::error::FileCommandError;

use crate::i18n::{t, t_with};
pub use read::read;
use save::Outcome;

const POLL: Duration = Duration::from_millis(20);
const RETRY: Duration = Duration::from_secs(5);

pub struct Note {
    pub path: PathBuf,
    hash: Arc<Mutex<String>>,
    pending: Option<Instant>,
    pub save_error: Option<String>,
    replaced: Option<crop::Rope>,
    queued: usize,
    done: (Sender<Outcome>, Receiver<Outcome>),
}

impl Note {
    pub fn open(path: PathBuf) -> Result<(Note, crop::Rope), FileCommandError> {
        let loaded = read(&path)?;
        let note = Note { path, hash: Arc::new(Mutex::new(loaded.hash)), pending: None, save_error: None, replaced: None, queued: 0, done: channel() };
        Ok((note, loaded.text))
    }

    pub fn edit(&mut self, debounce: Duration) {
        self.pending = Some(Instant::now() + debounce);
    }

    pub fn deadline(&self) -> Option<Instant> {
        if self.queued > 0 { Some(Instant::now() + POLL) } else { self.pending }
    }

    fn queue(&mut self, core: &Arc<Core>, text: &crop::Rope) {
        self.pending = None;
        self.queued += 1;
        save::queue(Arc::clone(core), self.path.clone(), text.clone(), Arc::clone(&self.hash), self.done.0.clone());
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

    pub fn flush(&mut self, core: &Arc<Core>, text: &crop::Rope) {
        if self.pending.is_some() {
            self.queue(core, text);
        }
        crate::disk::wait();
        self.collect();
    }

    pub fn leave(mut self, core: &Arc<Core>, text: &crop::Rope) {
        if self.pending.is_some() {
            self.queue(core, text);
        }
    }

    pub fn take_replaced(&mut self) -> Option<crop::Rope> {
        self.replaced.take()
    }

    pub fn retry(&mut self) {
        self.pending = Some(Instant::now());
    }

    pub fn reload(&mut self) -> Option<crop::Rope> {
        if self.pending.is_some() || self.queued > 0 {
            return None;
        }
        let loaded = read(&self.path).ok()?;
        let mut hash = self.hash.lock().unwrap_or_else(PoisonError::into_inner);
        if loaded.hash == *hash {
            return None;
        }
        *hash = loaded.hash;
        Some(loaded.text)
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
        std::fs::write(&path, "два").unwrap();
        assert_eq!(note.reload().map(|text| text.to_string()).as_deref(), Some("два"));
        note.edit(Duration::ZERO);
        assert!(note.deadline().is_some());
        assert_eq!(note.reload(), None, "несохранённая правка не затирается");
    }
}
