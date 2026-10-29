use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aquilum_core::files::models::{FileItem, FileItemType};
use aquilum_core::files::workspace::read_directory_impl;

use super::comparable;
use super::vault::walk;

#[derive(Default)]
pub struct Counts {
    own: BTreeMap<String, u32>,
}

fn read(dir: &Path) -> (u32, Vec<PathBuf>) {
    let items: Vec<FileItem> = read_directory_impl(dir).unwrap_or_default();
    let folders = items.iter().filter(|item| item.item_type == FileItemType::Folder).map(|item| PathBuf::from(&item.id)).collect();
    (items.len() as u32, folders)
}

fn subtree(key: &str) -> std::ops::Range<String> {
    format!("{key}/")..format!("{key}0")
}

impl Counts {
    pub fn scan(dir: &Path) -> Self {
        let mut counts = Counts::default();
        counts.add(dir);
        counts
    }

    fn add(&mut self, dir: &Path) {
        walk(dir, |key, items| {
            self.own.insert(key, items.len() as u32);
        });
    }

    fn forget(&mut self, key: &str) {
        self.own.remove(key);
        let nested: Vec<String> = self.own.range(subtree(key)).map(|(k, _)| k.clone()).collect();
        for nested in nested {
            self.own.remove(&nested);
        }
    }

    pub fn knows(&self, dir: &Path) -> bool {
        self.own.contains_key(&comparable(dir))
    }

    pub fn refresh(&mut self, dir: &Path) {
        let key = comparable(dir);
        if !dir.is_dir() {
            self.forget(&key);
            return;
        }
        let (items, folders) = read(dir);
        self.own.insert(key.clone(), items);
        let present: Vec<String> = folders.iter().map(|f| comparable(f)).collect();
        let prefix_len = key.len() + 1;
        let gone: Vec<String> = self.own.range(subtree(&key)).map(|(k, _)| k).filter(|k| !k[prefix_len..].contains('/') && !present.contains(k)).cloned().collect();
        for key in gone {
            self.forget(&key);
        }
        for folder in folders {
            if !self.knows(&folder) {
                self.add(&folder);
            }
        }
    }

    pub fn total(&self, dir: &Path) -> Option<u32> {
        let key = comparable(dir);
        let own = *self.own.get(&key)?;
        Some(own + self.own.range(subtree(&key)).map(|(_, items)| items).sum::<u32>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::file_tree::model::tests::tree;

    #[test]
    fn the_scan_counts_every_folder_and_a_refresh_adds_a_new_subtree() {
        let (dir, _model) = tree();
        let mut counts = Counts::scan(dir.path());
        assert_eq!(counts.total(dir.path()), Some(9));
        assert_eq!(counts.total(&dir.path().join("Проекты")), Some(3));
        std::fs::create_dir_all(dir.path().join("Проекты/Новая")).unwrap();
        std::fs::write(dir.path().join("Проекты/Новая/файл.md"), "").unwrap();
        counts.refresh(&dir.path().join("Проекты"));
        assert_eq!(counts.total(dir.path()), Some(11));
    }
}
