use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};

use aquilum_core::Core;
use aquilum_core::documents::conflict::{ConflictCause, ConflictReport};
use aquilum_core::documents::io::preserve_copy;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

use super::read::read;

pub enum Outcome {
    Saved,
    Copied { copy: String, disk: Option<crop::Rope> },
    Unsaved,
    Failed(String),
}

fn write(core: &Core, path: &PathBuf, text: &crop::Rope, hash: &Mutex<String>) -> Outcome {
    let _zone = aq_trace::zone("заметка: сохранение");
    let body = text.to_string();
    let expected = hash.lock().unwrap_or_else(PoisonError::into_inner).clone();
    match gate::write(core, path, &body, Some(&expected), Source::Me, None) {
        Ok(result) => {
            *hash.lock().unwrap_or_else(PoisonError::into_inner) = result.hash;
            Outcome::Saved
        }
        Err(FileCommandError::Conflict { .. }) => {
            let report = ConflictReport { cause: ConflictCause::Diverged, disk_hash: None, synced_hash: Some(expected) };
            let offset = i64::from(chrono::Local::now().offset().local_minus_utc());
            let Some(copy) = preserve_copy(core, path, &body, report, offset) else { return Outcome::Unsaved };
            let disk = read(path).ok().map(|loaded| {
                *hash.lock().unwrap_or_else(PoisonError::into_inner) = loaded.hash;
                loaded.text
            });
            Outcome::Copied { copy: copy.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), disk }
        }
        Err(error) => {
            report!("{} не сохранён: {error:?}", path.display());
            Outcome::Failed(format!("{error}"))
        }
    }
}

pub fn queue(core: Arc<Core>, path: PathBuf, text: crop::Rope, hash: Arc<Mutex<String>>, done: Sender<Outcome>) {
    crate::disk::run(move || {
        let _ = done.send(write(&core, &path, &text, &hash));
    });
}
