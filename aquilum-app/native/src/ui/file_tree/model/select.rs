use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{Modifiers, Press, TreeModel, comparable};

impl TreeModel {
    pub fn press(&mut self, index: usize, modifiers: Modifiers) -> Option<Press> {
        let row = self.rows.get(index)?.clone();
        if modifiers.action && modifiers.shift && !row.folder {
            self.selected.clear();
            self.anchor = Some(row.path.clone());
            return Some(Press::Open { path: row.path, new_tab: true });
        }
        if modifiers.shift {
            self.select_range(index);
            self.rebuild();
            return Some(Press::Rebuild);
        }
        if modifiers.action {
            let key = comparable(&row.path);
            if !self.selected.remove(&key) {
                self.selected.insert(key);
            }
            self.anchor = Some(row.path);
            self.rebuild();
            return Some(Press::Rebuild);
        }
        if row.folder {
            let key = comparable(&row.path);
            self.selected = HashSet::from([key.clone()]);
            self.anchor = Some(row.path.clone());
            if self.filter.is_none() && self.expanded.remove(&key).is_none() {
                self.expanded.insert(key.clone(), row.path.clone());
                if !self.directories.contains_key(&key) {
                    self.load(&row.path);
                }
            }
            self.rebuild();
            return Some(Press::Rebuild);
        }
        self.selected.clear();
        self.anchor = Some(row.path.clone());
        Some(Press::Open { path: row.path, new_tab: false })
    }

    pub fn focus(&mut self, index: usize) -> Vec<PathBuf> {
        let Some(row) = self.rows.get(index).cloned() else { return Vec::new() };
        let key = comparable(&row.path);
        if !(self.selected.len() > 1 && self.selected.contains(&key)) {
            self.selected = HashSet::from([key]);
            self.anchor = Some(row.path.clone());
        }
        self.rebuild();
        self.rows.iter().filter(|r| r.selected).map(|r| r.path.clone()).collect()
    }

    pub fn drag_sources(&self, index: usize) -> Vec<PathBuf> {
        let Some(row) = self.rows.get(index) else { return Vec::new() };
        if self.selected.len() > 1 && self.selected.contains(&comparable(&row.path)) {
            self.rows.iter().filter(|r| r.selected).map(|r| r.path.clone()).collect()
        } else {
            vec![row.path.clone()]
        }
    }

    pub fn drop_target(&self, source: &Path, index: usize) -> Option<PathBuf> {
        let row = self.rows.get(index)?;
        let target = if row.folder { row.path.clone() } else { row.path.parent()?.to_path_buf() };
        let (source_key, target_key) = (comparable(source), comparable(&target));
        if comparable(&row.path) == source_key || source.parent().map(comparable).as_deref() == Some(target_key.as_str()) {
            return None;
        }
        let inside = target_key == source_key || target_key.starts_with(&format!("{source_key}/"));
        (!inside).then_some(target)
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.rebuild();
    }

    fn select_range(&mut self, to: usize) {
        let anchor = self.anchor.as_deref().map(comparable);
        let from = anchor.and_then(|a| self.rows.iter().position(|r| comparable(&r.path) == a));
        let Some(from) = from else {
            self.selected = HashSet::from([comparable(&self.rows[to].path)]);
            self.anchor = Some(self.rows[to].path.clone());
            return;
        };
        let (start, end) = if from < to { (from, to) } else { (to, from) };
        self.selected = self.rows[start..=end].iter().map(|r| comparable(&r.path)).collect();
    }
}
