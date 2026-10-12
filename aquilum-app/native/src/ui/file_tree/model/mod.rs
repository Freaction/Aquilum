mod edit;
mod load;
mod select;
mod sort;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use aquilum_core::files::models::FileItem;

pub use sort::Sort;

pub fn comparable(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub path: PathBuf,
    pub name: String,
    pub folder: bool,
    pub depth: usize,
    pub expanded: bool,
    pub guides: Vec<usize>,
    pub active: bool,
    pub selected: bool,
}

#[derive(Debug, PartialEq)]
pub enum Press {
    Rebuild,
    Open { path: PathBuf, new_tab: bool },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub action: bool,
    pub shift: bool,
}

#[derive(Default)]
pub struct TreeModel {
    root: Option<PathBuf>,
    directories: HashMap<String, Vec<FileItem>>,
    expanded: BTreeMap<String, PathBuf>,
    selected: HashSet<String>,
    anchor: Option<PathBuf>,
    active: Option<PathBuf>,
    renaming: Option<PathBuf>,
    rows: Vec<Row>,
    sort: Sort,
    times: HashMap<String, (i128, i128)>,
}

impl TreeModel {
    pub fn new(root: Option<PathBuf>, expanded: impl IntoIterator<Item = PathBuf>) -> Self {
        let expanded = expanded.into_iter().map(|p| (comparable(&p), p)).collect();
        let mut model = TreeModel { root, expanded, ..Default::default() };
        model.reload_all();
        model
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn expanded_paths(&self) -> Vec<PathBuf> {
        self.expanded.values().cloned().collect()
    }

    pub fn sort(&self) -> Sort {
        self.sort
    }

    pub fn collapse_all(&mut self) -> bool {
        if self.expanded.is_empty() {
            return false;
        }
        self.expanded.clear();
        self.rebuild();
        true
    }

    pub fn target_folder(&self) -> Option<PathBuf> {
        let folder = (self.selected.len() == 1)
            .then(|| self.rows.iter().find(|r| r.folder && self.selected.contains(&comparable(&r.path))))
            .flatten();
        folder.map(|row| row.path.clone()).or_else(|| self.root.clone())
    }

    pub fn index_of(&self, path: &Path) -> Option<usize> {
        let key = comparable(path);
        self.rows.iter().position(|r| comparable(&r.path) == key)
    }

    pub fn set_active(&mut self, path: Option<PathBuf>) {
        self.anchor = path.clone();
        self.selected.clear();
        self.active = path;
        self.rebuild();
    }

    pub fn renaming(&self) -> Option<&Path> {
        self.renaming.as_deref()
    }

    pub fn start_rename(&mut self, path: PathBuf) {
        self.renaming = Some(path);
    }

    pub fn stop_rename(&mut self) {
        self.renaming = None;
    }
}
