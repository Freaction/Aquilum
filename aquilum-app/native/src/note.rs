//! Открытая заметка и её сохранение — как `useAutoSave` во фронтенде: правка ждёт паузы
//! `editor.saveDebounceMs` и пишется через ворота ядра с хешем того, что было на диске.
//! Если файл успели изменить снаружи, запись отклоняется, а не затирает чужую правку.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use aquilum_core::Core;
use aquilum_core::files::document::read_file_snapshot_impl;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

pub struct Note {
    pub path: PathBuf,
    /// Хеш байтов файла на диске после последнего чтения или записи.
    hash: String,
    /// Несохранённый текст и когда его записать.
    pending: Option<(String, Instant)>,
}

impl Note {
    /// Читает заметку; возвращает её и текст для редактора.
    pub fn open(path: PathBuf) -> Result<(Note, String), FileCommandError> {
        let snapshot = read_file_snapshot_impl(&path)?;
        Ok((Note { path, hash: snapshot.hash, pending: None }, snapshot.content))
    }

    /// Текст в редакторе изменился: запись откладывается на `debounce`.
    pub fn edit(&mut self, text: String, debounce: Duration) {
        self.pending = Some((text, Instant::now() + debounce));
    }

    /// Когда записать отложенную правку.
    pub fn deadline(&self) -> Option<Instant> {
        self.pending.as_ref().map(|(_, at)| *at)
    }

    /// Пишет отложенную правку, если она есть.
    pub fn save(&mut self, core: &Core) {
        let Some((text, _)) = self.pending.take() else { return };
        match gate::write(core, &self.path, &text, Some(&self.hash), Source::Me, None) {
            Ok(result) => self.hash = result.hash,
            Err(FileCommandError::Conflict { .. }) => {
                // Как фронтенд при конфликте: на диске чужая версия, правка остаётся только в
                // редакторе. Слияние появится вместе с редактором на yrs.
                eprintln!("{} изменён снаружи — правка не записана", self.path.display());
            }
            Err(error) => {
                eprintln!("{} не сохранён: {error:?}", self.path.display());
                self.pending = Some((text, Instant::now() + Duration::from_secs(5)));
            }
        }
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
        self.path == path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_ignores_own_write_and_sees_external_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.md");
        std::fs::write(&path, "раз").unwrap();
        let (mut note, text) = Note::open(path.clone()).unwrap();
        assert_eq!(text, "раз");
        assert_eq!(note.reload(), None);
        std::fs::write(&path, "два").unwrap();
        assert_eq!(note.reload().as_deref(), Some("два"));
        note.edit("три".into(), Duration::ZERO);
        assert!(note.deadline().is_some());
        assert_eq!(note.reload(), None, "несохранённая правка не затирается");
    }
}
