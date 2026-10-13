//! Открытая заметка и её сохранение — как `useAutoSave` во фронтенде: правка ждёт паузы
//! `editor.saveDebounceMs` и пишется через ворота ядра с хешем того, что было на диске.
//! Если файл успели изменить снаружи, запись отклоняется, а не затирает чужую правку.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use aquilum_core::Core;
use aquilum_core::documents::conflict::{ConflictCause, ConflictReport};
use aquilum_core::documents::io::preserve_copy;
use aquilum_core::files::document::read_file_snapshot_impl;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

use crate::i18n::{t, t_with};

pub struct Note {
    pub path: PathBuf,
    /// Хеш байтов файла на диске после последнего чтения или записи.
    hash: String,
    /// Когда записать несохранённую правку.
    pending: Option<Instant>,
    pub save_error: Option<String>,
    replaced: Option<String>,
}

impl Note {
    /// Читает заметку; возвращает её и текст для редактора.
    pub fn open(path: PathBuf) -> Result<(Note, String), FileCommandError> {
        let snapshot = read_file_snapshot_impl(&path)?;
        Ok((Note { path, hash: snapshot.hash, pending: None, save_error: None, replaced: None }, snapshot.content))
    }

    /// Текст в редакторе изменился: запись откладывается на `debounce`.
    pub fn edit(&mut self, debounce: Duration) {
        self.pending = Some(Instant::now() + debounce);
    }

    /// Когда записать отложенную правку.
    pub fn deadline(&self) -> Option<Instant> {
        self.pending
    }

    /// Пишет отложенную правку, если она есть.
    pub fn save(&mut self, core: &Core, text: &str) -> Result<(), String> {
        if self.pending.take().is_none() {
            return Ok(());
        }
        match gate::write(core, &self.path, text, Some(&self.hash), Source::Me, None) {
            Ok(result) => {
                self.hash = result.hash;
                self.save_error = None;
                Ok(())
            }
            Err(FileCommandError::Conflict { .. }) => {
                let report = ConflictReport { cause: ConflictCause::Diverged, disk_hash: None, synced_hash: Some(self.hash.clone()) };
                let offset = i64::from(chrono::Local::now().offset().local_minus_utc());
                let Some(copy) = preserve_copy(core, &self.path, text, report, offset) else {
                    self.pending = Some(Instant::now() + Duration::from_secs(5));
                    return self.fail(t("editor.conflictUnsaved"));
                };
                if let Ok(snapshot) = read_file_snapshot_impl(&self.path) {
                    self.hash = snapshot.hash;
                    self.replaced = Some(snapshot.content);
                }
                let name = copy.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                self.fail(t_with("editor.conflictCopy", &[("name", &name)]))
            }
            Err(error) => {
                report!("{} не сохранён: {error:?}", self.path.display());
                self.pending = Some(Instant::now() + Duration::from_secs(5));
                self.fail(t_with("editor.saveFailed", &[("reason", &format!("{error}"))]))
            }
        }
    }

    fn fail(&mut self, message: String) -> Result<(), String> {
        self.save_error = Some(message.clone());
        Err(message)
    }

    pub fn take_replaced(&mut self) -> Option<String> {
        self.replaced.take()
    }

    /// Повторить попытку сохранения прямо сейчас.
    pub fn retry_save(&mut self, core: &Core, text: &str) -> Result<(), String> {
        self.save(core, text)
    }

    /// Файл изменился на диске. Без несохранённой правки возвращает новый текст, если это не
    /// наша же запись.
    pub fn reload(&mut self) -> Option<String> {
        if self.pending.is_some() {
            return None;
        }
        let snapshot = read_file_snapshot_impl(&self.path).ok()?;
        if snapshot.hash == self.hash {
            return None;
        }
        self.hash = snapshot.hash;
        Some(snapshot.content)
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
        assert_eq!(note.reload().as_deref(), Some("два"));
        note.edit(Duration::ZERO);
        assert!(note.deadline().is_some());
        assert_eq!(note.reload(), None, "несохранённая правка не затирается");
    }
}
