use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aquilum_core::files::document::write_file_atomic_impl;
use serde::{Deserialize, Serialize};

use crate::session::relative;
use crate::ui::file_tree::model::comparable;

const MAX_ENTRIES: usize = 100;

#[derive(Default, Deserialize, Serialize)]
struct Stored {
    entries: Vec<String>,
    index: i64,
}

pub struct Navigation {
    file: PathBuf,
    root: Option<PathBuf>,
    entries: Vec<PathBuf>,
    index: Option<usize>,
}

fn load_all(file: &Path) -> HashMap<String, Stored> {
    std::fs::read_to_string(file).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default()
}

impl Navigation {
    pub fn load(data_dir: &Path, root: Option<&Path>) -> Self {
        let file = data_dir.join("navigation.json");
        let mut navigation = Navigation { file, root: root.map(Path::to_path_buf), entries: Vec::new(), index: None };
        let Some(root) = root else { return navigation };
        if let Some(stored) = load_all(&navigation.file).remove(&comparable(root)) {
            navigation.entries = stored.entries.iter().map(|rel| root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))).collect();
            if !navigation.entries.is_empty() {
                navigation.index = Some(stored.index.clamp(0, navigation.entries.len() as i64 - 1) as usize);
            }
        }
        navigation
    }

    pub fn push(&mut self, path: &Path) {
        if self.index.and_then(|i| self.entries.get(i)).is_some_and(|current| comparable(current) == comparable(path)) {
            return;
        }
        self.entries.truncate(self.index.map_or(0, |i| i + 1));
        self.entries.push(path.to_path_buf());
        if self.entries.len() > MAX_ENTRIES {
            self.entries.drain(..self.entries.len() - MAX_ENTRIES);
        }
        self.index = Some(self.entries.len() - 1);
        self.save();
    }

    pub fn step(&mut self, delta: isize) -> Option<PathBuf> {
        let next = self.index?.checked_add_signed(delta).filter(|&i| i < self.entries.len())?;
        self.index = Some(next);
        self.save();
        self.entries.get(next).cloned()
    }

    pub fn can_back(&self) -> bool {
        self.index.is_some_and(|i| i > 0)
    }

    pub fn can_forward(&self) -> bool {
        self.index.is_some_and(|i| i + 1 < self.entries.len())
    }

    fn save(&self) {
        let Some(root) = &self.root else { return };
        let mut all = load_all(&self.file);
        let entries = self.entries.iter().filter_map(|p| relative(root, p)).collect();
        all.insert(comparable(root), Stored { entries, index: self.index.map_or(-1, |i| i as i64) });
        let Ok(json) = serde_json::to_string(&all) else { return };
        if let Err(e) = write_file_atomic_impl(&self.file, &json, None) {
            eprintln!("история переходов не сохранена: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_step_and_persist() {
        let data = tempfile::tempdir().unwrap();
        let root = data.path().join("vault");
        let a = root.join("a.md");
        let b = root.join("b.md");
        let c = root.join("c.md");
        let mut nav = Navigation::load(data.path(), Some(&root));
        assert!(!nav.can_back() && !nav.can_forward());
        nav.push(&a);
        nav.push(&a);
        nav.push(&b);
        nav.push(&c);
        assert_eq!(nav.step(-1), Some(b.clone()));
        assert_eq!(nav.step(-1), Some(a.clone()));
        assert_eq!(nav.step(-1), None);
        assert!(nav.can_forward());
        nav.push(&c);
        assert!(!nav.can_forward());
        let nav = Navigation::load(data.path(), Some(&root));
        assert!(nav.can_back());
        assert_eq!(nav.entries, vec![a, c]);
    }
}
