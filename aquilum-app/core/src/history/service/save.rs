use super::{locate, report, HistoryService, Roots};
use crate::files::document::hash_file;
use crate::files::error::FileCommandError;
use crate::history::delta::{self, compose, single};
use crate::history::milestone::{self, Incoming, MilestonePlan, Recorded, Source};
use crate::history::rebuild::{is_delta, rebuild};
use crate::history::store::{self, VERSION_EXTENSION};
use crate::text::content::{self, Content, Splice};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct Change<'a> {
    pub splice: Splice,
    pub base: &'a str,
}

pub struct NoteWrite<'a> {
    pub text: &'a dyn Content,
    pub hash: &'a str,
    pub source: Source,
    pub name: Option<&'a str>,
    pub change: Option<Change<'a>>,
    pub original: Option<PathBuf>,
}

impl HistoryService {
    pub fn original(&self, roots: Roots<'_>, note: &Path) -> Option<PathBuf> {
        let metadata = fs::metadata(note).ok().filter(|metadata| metadata.is_file() && metadata.len() > 0)?;
        let at = metadata.modified().unwrap_or_else(|_| SystemTime::now());
        let mut state = self.lock();
        let located = locate(&mut state, roots, note)?;
        if !store::versions(&located.dir).is_empty() {
            return None;
        }
        let path = store::version_path(&located.dir, &self.device, Source::Start, at, VERSION_EXTENSION);
        match fs::create_dir_all(&located.dir).and_then(|()| fs::copy(note, &path)) {
            Ok(_) => Some(path),
            Err(error) => {
                report(&located.dir, "не сохранила исходный текст", &error.to_string());
                None
            }
        }
    }

    pub fn saved(&self, roots: Roots<'_>, note: &Path, write: NoteWrite<'_>) -> bool {
        let now = SystemTime::now();
        let mut state = self.lock();
        let Some(located) = locate(&mut state, roots, note) else {
            discard(write.original.as_deref());
            return false;
        };
        let entry = state.notes.entry(located.key).or_default();
        let dir = located.dir.as_path();
        let original = write.original.as_deref().and_then(Path::file_name).map(|name| name.to_string_lossy().into_owned());
        let versions = store::versions(dir).into_iter().filter(|version| Some(&version.id) != original.as_ref()).collect::<Vec<_>>();
        let latest = versions.last();
        let new_len = write.text.byte_len();
        let same_as_latest = latest.is_some_and(|version| matches(dir, &version.id, new_len, || Some(write.hash.to_owned())));
        let base = latest.map(|version| version.id.clone()).or_else(|| original.clone());
        let change = match base.as_deref().filter(|_| !write.source.is_quiet() && !same_as_latest) {
            Some(base) => change_from(dir, base, &write).map_err(|error| report(dir, "не сравнила версию", &error.to_string())).ok(),
            None => None,
        };
        let latest_inserted = latest.filter(|version| is_delta(&version.id)).and_then(|version| delta::read(&dir.join(&version.id)).ok()).map(|delta| delta.inserted());
        let incoming = Incoming {
            source: write.source,
            same_as_latest,
            same_as_previous: versions.len() >= 2 && matches(dir, &versions[versions.len() - 2].id, new_len, || Some(write.hash.to_owned())),
            original_differs: original.as_deref().is_some_and(|id| !matches(dir, id, new_len, || Some(write.hash.to_owned()))),
            small: change.is_some_and(|(_, small)| small),
            fits_latest: latest_inserted.zip(change).is_some_and(|(inserted, (splice, _))| milestone::fits(inserted, splice)),
        };
        let names = store::names(dir);
        let plan = milestone::plan(&self.device, &incoming, &Recorded { versions: &versions, names: &names, last_edit: *entry }, now);
        if !matches!(plan, MilestonePlan::CreateNew { keep_original: true }) {
            discard(write.original.as_deref());
        }
        let splice = change.map(|(splice, _)| splice);
        let id = match plan {
            MilestonePlan::Quiet => return false,
            MilestonePlan::Skip => {
                let Some((name, latest)) = write.name.zip(latest) else { return false };
                self.write_name(dir, &latest.id, name, now);
                return true;
            }
            MilestonePlan::RevertToPrevious { remove_id } => {
                store::remove_version_and_names(dir, &remove_id);
                *entry = None;
                return true;
            }
            MilestonePlan::UpdateLatest { id } => {
                *entry = Some(now);
                update(dir, &id, splice, &write).map_err(|error| report(dir, "не обновила версию", &error.to_string())).ok().map(|()| id)
            }
            MilestonePlan::CreateNew { keep_original } => {
                *entry = Some(now);
                let base = latest.map(|version| version.id.clone()).or(original.filter(|_| keep_original));
                self.create(dir, base.as_deref().zip(splice), &write, now)
            }
        };
        if let (Some(id), Some(name)) = (id, write.name) {
            self.write_name(dir, &id, name, now);
        }
        true
    }

    fn create(&self, dir: &Path, base: Option<(&str, Splice)>, write: &NoteWrite<'_>, now: SystemTime) -> Option<String> {
        let written = match base {
            Some((base, splice)) => {
                let path = store::version_path(dir, &self.device, write.source, now, delta::EXTENSION);
                delta::write(&path, Some(base), write.hash, &single(splice), write.text).map(|()| path)
            }
            None => store::write_version(dir, &self.device, write.source, now, write.text),
        };
        match written {
            Ok(path) => path.file_name().map(|name| name.to_string_lossy().into_owned()),
            Err(error) => {
                report(dir, "не записала версию", &error.to_string());
                None
            }
        }
    }
}

fn update(dir: &Path, id: &str, splice: Option<Splice>, write: &NoteWrite<'_>) -> Result<(), FileCommandError> {
    let path = dir.join(id);
    let old = delta::read(&path)?;
    let Some(splice) = splice else {
        return Err(FileCommandError::Task { message: "нет правки для версии".to_owned() });
    };
    delta::write(&path, old.base.as_deref(), write.hash, &compose(&old.regions, splice), write.text)
}

fn change_from(dir: &Path, base: &str, write: &NoteWrite<'_>) -> io::Result<(Splice, bool)> {
    let old = rebuild(dir, base)?;
    let hinted = write.change.as_ref().filter(|change| {
        (write.text.byte_len() + change.splice.removed)
            .checked_sub(change.splice.inserted)
            .is_some_and(|len| len == old.byte_len() && matches(dir, base, len, || Some(change.base.to_owned())))
    });
    let splice = match hinted {
        Some(change) => change.splice,
        None => content::splice(&old, write.text)?,
    };
    Ok((splice, milestone::is_small(&old, write.text, splice)?))
}

pub(super) fn matches(dir: &Path, id: &str, len: usize, hash: impl FnOnce() -> Option<String>) -> bool {
    let path = dir.join(id);
    if is_delta(id) {
        return delta::read(&path).is_ok_and(|delta| delta.len == len && hash().as_deref() == Some(delta.hash.as_str()));
    }
    fs::metadata(&path).is_ok_and(|metadata| metadata.len() as usize == len) && hash() == hash_file(&path).ok()
}

fn discard(original: Option<&Path>) {
    if let Some(path) = original {
        let _ = fs::remove_file(path);
    }
}
