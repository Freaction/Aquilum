mod read;
mod save;
#[cfg(test)]
mod tests;

use super::store;
use crate::files::relocate::{move_folder, remove_empty_ancestors, Placement};
use crate::files::trash::trash_path;
use crate::search::paths::{identity, strip_root};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

pub use read::{HistoryPage, Reverting, VersionTexts, VersionView};
pub use save::{Change, NoteWrite};

const DEVICE_FILE: &str = "device-id";

pub struct HistoryService {
    device: String,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    roots: Vec<PathBuf>,
    notes: HashMap<String, Option<SystemTime>>,
}

struct Located {
    key: String,
    vault: PathBuf,
    dir: PathBuf,
}

pub type Roots<'a> = &'a dyn Fn() -> Vec<PathBuf>;

impl HistoryService {
    pub fn new(app_data: &Path) -> Self {
        Self {
            device: device_id(app_data),
            state: Mutex::default(),
        }
    }

    pub fn moved(&self, roots: Roots<'_>, from: &Path, to: &Path) {
        let mut state = self.lock();
        let (Some(source), Some(target)) = (
            locate(&mut state, roots, from),
            locate(&mut state, roots, to),
        ) else {
            return;
        };
        let note = state.notes.remove(&source.key);
        state.notes.remove(&target.key);
        move_history(&source.dir, &target.dir, Placement::Workspace, &source.vault);
        if let Some(note) = note {
            state.notes.insert(target.key, note);
        }
    }

    pub fn trashed(&self, roots: Roots<'_>, note: &Path, trashed_at: &Path) {
        let mut state = self.lock();
        let Some(located) = locate(&mut state, roots, note) else {
            return;
        };
        let note = state.notes.remove(&located.key);
        let Some((_, target)) = trash_history(&located.vault, trashed_at) else {
            return;
        };
        move_history(&located.dir, &target, Placement::Trash, &located.vault);
        if let Some(note) = note {
            state.notes.insert(identity(trashed_at), note);
        }
    }

    pub fn restored(&self, roots: Roots<'_>, trashed_at: &Path, restored: &Path) {
        let mut state = self.lock();
        let Some(located) = locate(&mut state, roots, restored) else {
            return;
        };
        let note = state.notes.remove(&identity(trashed_at));
        state.notes.remove(&located.key);
        let Some((trash, source)) = trash_history(&located.vault, trashed_at) else {
            return;
        };
        move_history(&source, &located.dir, Placement::Workspace, &trash);
        if let Some(note) = note {
            state.notes.insert(located.key, note);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn locate(state: &mut State, roots: Roots<'_>, note: &Path) -> Option<Located> {
    let find = |roots: &[PathBuf]| {
        roots.iter().find_map(|root| {
            relative_inside(root, note).map(|relative| (root.clone(), relative))
        })
    };
    let (vault, relative) = find(&state.roots).or_else(|| {
        state.roots = roots();
        find(&state.roots)
    })?;
    Some(Located {
        key: identity(note),
        dir: store::history_dir(&vault, &relative),
        vault,
    })
}

fn trash_history(vault: &Path, trashed_at: &Path) -> Option<(PathBuf, PathBuf)> {
    let trash = trash_path(vault);
    let dir = store::history_dir(&trash, &relative_inside(&trash, trashed_at)?);
    Some((trash, dir))
}

fn relative_inside(root: &Path, path: &Path) -> Option<PathBuf> {
    strip_root(root, path)
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

fn move_history(from: &Path, to: &Path, placement: Placement, base: &Path) {
    match move_folder(from, to, placement) {
        Ok(()) => remove_empty_ancestors(from, base),
        Err(error) => report(from, "не перенесена", &error.to_string()),
    }
}

fn report(dir: &Path, what: &str, error: &str) {
    eprintln!("[aquilum:history] история {} {what}: {error}", dir.display());
}

fn device_id(app_data: &Path) -> String {
    let path = app_data.join(DEVICE_FILE);
    let stored = fs::read_to_string(&path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if let Some(device) = stored {
        return device;
    }
    let device = uuid::Uuid::new_v4().simple().to_string()[..8].to_owned();
    if let Err(error) = fs::create_dir_all(app_data).and_then(|()| fs::write(&path, &device)) {
        eprintln!("[aquilum:history] идентификатор устройства не сохранён: {error}");
    }
    device
}
