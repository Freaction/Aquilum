use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::Sort;
use aquilum_core::search::paths::identity;

pub(super) struct ExpandedStore {
    path: PathBuf,
    workspaces: BTreeMap<String, Vec<String>>,
    sorts: BTreeMap<String, Sort>,
    colors: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Default, Deserialize, Serialize)]
struct StoreFile {
    workspaces: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    sorts: BTreeMap<String, Sort>,
    #[serde(default)]
    colors: BTreeMap<String, BTreeMap<String, String>>,
}

impl ExpandedStore {
    pub(super) fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("expanded-folders.json");
        let file = crate::state_file::load::<StoreFile>(&path).unwrap_or_default();
        ExpandedStore { path, workspaces: file.workspaces, sorts: file.sorts, colors: file.colors }
    }

    pub(super) fn sort(&self, root: &Path) -> Sort {
        self.sorts.get(&identity(root)).copied().unwrap_or_default()
    }

    pub(super) fn set_sort(&mut self, root: &Path, sort: Sort) {
        self.sorts.insert(identity(root), sort);
        self.save();
    }

    pub(super) fn colors(&self, root: &Path) -> Vec<(PathBuf, String)> {
        self.colors.get(&identity(root)).into_iter().flatten().map(|(relative, name)| (root.join(relative), name.clone())).collect()
    }

    pub(super) fn set_colors(&mut self, root: &Path, colors: &[(PathBuf, &str)]) {
        let relative = colors
            .iter()
            .filter_map(|(path, name)| Some((path.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/"), (*name).to_owned())))
            .collect();
        self.colors.insert(identity(root), relative);
        self.save();
    }

    pub(super) fn get(&self, root: &Path) -> Vec<PathBuf> {
        self.workspaces
            .get(&identity(root))
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
        self.workspaces.insert(identity(root), relative);
        self.save();
    }

    fn save(&self) {
        let file = StoreFile { workspaces: self.workspaces.clone(), sorts: self.sorts.clone(), colors: self.colors.clone() };
        crate::state_file::save(&self.path, &file, "раскрытые папки");
    }
}
