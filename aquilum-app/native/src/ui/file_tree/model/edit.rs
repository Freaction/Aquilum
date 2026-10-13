use std::path::{Path, PathBuf};

use super::{TreeModel, comparable};

fn shifted(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    let rest = path.strip_prefix(from).ok()?;
    Some(if rest.as_os_str().is_empty() { to.to_path_buf() } else { to.join(rest) })
}

impl TreeModel {
    pub fn moved(&mut self, sources: &[PathBuf], target: &Path) {
        for source in sources {
            if let Some(name) = source.file_name() {
                self.relocate(source, &target.join(name));
            }
        }
        self.rebuild();
    }

    fn relocate(&mut self, from: &Path, to: &Path) {
        let moved: Vec<(String, PathBuf)> = self.expanded.iter().filter_map(|(key, path)| Some((key.clone(), shifted(path, from, to)?))).collect();
        for (key, path) in moved {
            self.expanded.remove(&key);
            self.expanded.insert(comparable(&path), path);
        }
        let painted: Vec<(String, PathBuf, &'static str)> =
            self.colors.iter().filter_map(|(key, (path, color))| Some((key.clone(), shifted(path, from, to)?, *color))).collect();
        for (key, path, color) in painted {
            self.colors.remove(&key);
            self.colors.insert(comparable(&path), (path, color));
        }
    }

    pub fn patch(&mut self, from: &Path, to: &Path, name: &str) {
        let from_key = comparable(from);
        for items in self.directories.values_mut() {
            for item in items.iter_mut().filter(|i| comparable(Path::new(&i.id)) == from_key) {
                item.id = to.to_string_lossy().into_owned();
                item.name = name.to_owned();
            }
        }
        self.relocate(from, to);
        if let Some(items) = self.directories.remove(&from_key) {
            self.directories.insert(comparable(to), items);
        }
        if self.active.as_deref().is_some_and(|a| comparable(a) == from_key) {
            self.active = Some(to.to_path_buf());
        }
        self.rebuild();
    }
}
