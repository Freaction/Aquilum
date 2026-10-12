use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::{Sort, comparable};

pub(super) struct ExpandedStore {
    path: PathBuf,
    workspaces: BTreeMap<String, Vec<String>>,
    sorts: BTreeMap<String, Sort>,
}

#[derive(Default, Deserialize, Serialize)]
struct StoreFile {
    workspaces: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    sorts: BTreeMap<String, Sort>,
}

impl ExpandedStore {
    pub(super) fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("expanded-folders.json");
        let file = std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<StoreFile>(&json).ok())
            .unwrap_or_default();
        ExpandedStore { path, workspaces: file.workspaces, sorts: file.sorts }
    }

    pub(super) fn sort(&self, root: &Path) -> Sort {
        self.sorts.get(&comparable(root)).copied().unwrap_or_default()
    }

    pub(super) fn set_sort(&mut self, root: &Path, sort: Sort) {
        self.sorts.insert(comparable(root), sort);
        self.save();
    }

    pub(super) fn get(&self, root: &Path) -> Vec<PathBuf> {
        self.workspaces
            .get(&comparable(root))
            .into_iter()
            .flatten()
            .map(|relative| root.join(relative))
            .collect()
    }

    pub(super) fn set(&mut self, root: &Path, expanded: &[PathBuf]) {
        let relative = expanded
            .iter()
            .filter_map(|p| p.strip_prefix(root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        self.workspaces.insert(comparable(root), relative);
        self.save();
    }

    fn save(&self) {
        let file = StoreFile { workspaces: self.workspaces.clone(), sorts: self.sorts.clone() };
        let Ok(json) = serde_json::to_string(&file) else { return };
        if let Err(e) = aquilum_core::files::document::write_file_atomic_impl(&self.path, &json, None) {
            report!("раскрытые папки не сохранены: {e}");
        }
    }
}
