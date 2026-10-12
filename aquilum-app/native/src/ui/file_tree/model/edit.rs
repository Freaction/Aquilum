use std::path::{Path, PathBuf};

use super::{TreeModel, comparable};

impl TreeModel {
    pub fn patch(&mut self, from: &Path, to: &Path, name: &str) {
        let from_key = comparable(from);
        for items in self.directories.values_mut() {
            for item in items.iter_mut().filter(|i| comparable(Path::new(&i.id)) == from_key) {
                item.id = to.to_string_lossy().into_owned();
                item.name = name.to_owned();
            }
        }
        let moved: Vec<(String, PathBuf)> = self
            .expanded
            .iter()
            .filter_map(|(key, path)| Some((key.clone(), to.join(path.strip_prefix(from).ok()?))))
            .collect();
        for (key, path) in moved {
            self.expanded.remove(&key);
            self.expanded.insert(comparable(&path), path);
        }
        if let Some(items) = self.directories.remove(&from_key) {
            self.directories.insert(comparable(to), items);
        }
        if self.active.as_deref().is_some_and(|a| comparable(a) == from_key) {
            self.active = Some(to.to_path_buf());
        }
        self.rebuild();
    }
}
