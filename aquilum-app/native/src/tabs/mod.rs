#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub id: Uuid,
    pub path: Option<PathBuf>,
    pub document_id: Option<Uuid>,
    pub graph: bool,
}

impl Tab {
    pub fn empty() -> Self {
        Tab { id: Uuid::new_v4(), path: None, document_id: None, graph: false }
    }

    pub fn title(&self) -> Option<String> {
        self.path.as_deref().and_then(Path::file_stem).map(|s| s.to_string_lossy().into_owned())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tabs {
    tabs: Vec<Tab>,
    active: Uuid,
}

impl Default for Tabs {
    fn default() -> Self {
        let tab = Tab::empty();
        Tabs { active: tab.id, tabs: vec![tab] }
    }
}

impl Tabs {
    pub fn restore(tabs: Vec<Tab>, active: Option<Uuid>) -> Self {
        if tabs.is_empty() {
            return Tabs::default();
        }
        let active = active.filter(|id| tabs.iter().any(|t| t.id == *id)).unwrap_or(tabs[0].id);
        Tabs { tabs, active }
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_id(&self) -> Uuid {
        self.active
    }

    pub fn active_index(&self) -> usize {
        self.tabs.iter().position(|t| t.id == self.active).unwrap_or(0)
    }

    pub fn active(&self) -> &Tab {
        &self.tabs[self.active_index()]
    }

    pub fn tab_mut(&mut self, id: Uuid) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    fn find_path(&self, path: &Path) -> Option<Uuid> {
        self.tabs.iter().find(|t| t.path.as_deref() == Some(path)).map(|t| t.id)
    }

    pub fn select(&mut self, id: Uuid) -> bool {
        if self.active == id || !self.tabs.iter().any(|t| t.id == id) {
            return false;
        }
        self.active = id;
        true
    }

    pub fn open(&mut self, path: PathBuf) {
        if let Some(id) = self.find_path(&path) {
            self.active = id;
            return;
        }
        let index = self.tabs.iter().position(|t| t.id == self.active);
        let tab = Tab { id: index.map_or_else(Uuid::new_v4, |i| self.tabs[i].id), path: Some(path), document_id: None, graph: false };
        self.active = tab.id;
        match index {
            Some(i) => self.tabs[i] = tab,
            None => self.tabs.push(tab),
        }
    }

    pub fn open_new(&mut self, path: PathBuf) {
        if let Some(id) = self.find_path(&path) {
            self.active = id;
            return;
        }
        let tab = Tab { id: Uuid::new_v4(), path: Some(path), document_id: None, graph: false };
        self.active = tab.id;
        self.tabs.push(tab);
    }

    pub fn open_graph(&mut self) {
        if let Some(tab) = self.tabs.iter().find(|t| t.graph) {
            self.active = tab.id;
            return;
        }
        let tab = Tab { graph: true, ..Tab::empty() };
        self.active = tab.id;
        self.tabs.push(tab);
    }

    pub fn has_graph(&self) -> bool {
        self.tabs.iter().any(|t| t.graph)
    }

    pub fn new_tab(&mut self) {
        let tab = Tab::empty();
        self.active = tab.id;
        self.tabs.push(tab);
    }

    pub fn close(&mut self, id: Uuid) {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else { return };
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.tabs.push(Tab::empty());
        }
        if id == self.active {
            self.active = self.tabs[index.saturating_sub(1).min(self.tabs.len() - 1)].id;
        }
    }

    pub fn reorder(&mut self, from: usize, to: usize) -> bool {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return false;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        true
    }

    pub fn rename(&mut self, from: &Path, to: &Path) -> bool {
        let mut changed = false;
        for tab in &mut self.tabs {
            if tab.path.as_deref() == Some(from) {
                tab.path = Some(to.to_path_buf());
                changed = true;
            }
        }
        changed
    }
}
