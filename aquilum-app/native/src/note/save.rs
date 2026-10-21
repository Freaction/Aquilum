use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use aquilum_core::Core;
use aquilum_core::documents::conflict::{ConflictCause, ConflictReport};
use aquilum_core::documents::io::preserve_copy;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate::{self, Saving};
use aquilum_core::history::{Change, Source};
use aquilum_core::history::delta::Region;

use super::{lock, Disk};
use super::read::{read, stamp};
use crate::rope::Text;

pub enum Outcome {
    Saved,
    Copied { copy: String, disk: Option<crop::Rope> },
    Unsaved,
    Failed(String),
}

pub struct Job {
    pub after: u64,
    pub seq: u64,
    pub change: Vec<Region>,
}

fn write(core: &Core, path: &PathBuf, text: &crop::Rope, job: &Job, disk: &Mutex<Disk>) -> Outcome {
    let _zone = aq_trace::zone("заметка: сохранение");
    let (expected, ours, hinted) = {
        let disk = lock(disk);
        (disk.hash.clone(), disk.stamp.is_some() && disk.stamp == stamp(path), disk.written == job.after)
    };
    let content = Text(text);
    let change = hinted.then_some(Change { regions: job.change.clone(), base: &expected });
    match gate::save(core, path, Saving { content: &content, expected_hash: (!ours).then_some(expected.as_str()), source: Source::Me, name: None, change }) {
        Ok(result) => {
            *lock(disk) = Disk { hash: result.hash, stamp: stamp(path), written: job.seq };
            Outcome::Saved
        }
        Err(FileCommandError::Conflict { .. }) => {
            let report = ConflictReport { cause: ConflictCause::Diverged, synced_hash: Some(expected) };
            let Some(copy) = preserve_copy(core, path, &content, report, crate::dates::offset_seconds()) else { return Outcome::Unsaved };
            let current = read(path).ok().map(|loaded| lock(disk).adopt(loaded));
            Outcome::Copied { copy: copy.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), disk: current }
        }
        Err(error) => {
            report!("{} не сохранён: {error:?}", path.display());
            Outcome::Failed(format!("{error}"))
        }
    }
}

pub(super) fn queue(core: Arc<Core>, path: PathBuf, text: crop::Rope, job: Job, disk: Arc<Mutex<Disk>>, done: Sender<Outcome>) {
    crate::disk::run(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| write(&core, &path, &text, &job, &disk)));
        let _ = done.send(outcome.unwrap_or_else(|_| Outcome::Failed("внутренняя ошибка записи".to_owned())));
    });
}
