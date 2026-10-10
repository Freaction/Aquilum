use std::path::{Path, PathBuf};

use aquilum_core::search::paths::is_markdown;
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
        Tab {
            id: Uuid::new_v4(),
            path: None,
            document_id: None,
            graph: false,
        }
    }

    pub fn base(path: PathBuf) -> Self {
        Tab {
            path: Some(path),
            ..Tab::empty()
        }
    }

    pub fn is_base(&self) -> bool {
        self.path
            .as_deref()
            .and_then(Path::extension)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("base"))
    }

    pub fn title(&self) -> Option<String> {
        self.path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
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
        Tabs {
            active: tab.id,
            tabs: vec![tab],
        }
    }
}

impl Tabs {
    pub fn restore(tabs: Vec<Tab>, active: Option<Uuid>) -> Self {
        if tabs.is_empty() {
            return Tabs::default();
        }
        let active = active
            .filter(|id| tabs.iter().any(|t| t.id == *id))
            .unwrap_or(tabs[0].id);
        Tabs { tabs, active }
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_id(&self) -> Uuid {
        self.active
    }

    pub fn active(&self) -> &Tab {
        self.tabs
            .iter()
            .find(|t| t.id == self.active)
            .unwrap_or(&self.tabs[0])
    }

    pub fn tab_mut(&mut self, id: Uuid) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    fn find_path(&self, path: &Path) -> Option<Uuid> {
        self.tabs
            .iter()
            .find(|t| t.path.as_deref() == Some(path))
            .map(|t| t.id)
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
        let tab = Tab {
            id: index.map_or_else(Uuid::new_v4, |i| self.tabs[i].id),
            path: Some(path),
            ..Tab::empty()
        };
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
        let tab = Tab {
            path: Some(path),
            ..Tab::empty()
        };
        self.active = tab.id;
        self.tabs.push(tab);
    }

    pub fn open_graph(&mut self) {
        if let Some(tab) = self.tabs.iter().find(|t| t.graph) {
            self.active = tab.id;
            return;
        }
        let tab = Tab {
            graph: true,
            ..Tab::empty()
        };
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
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return;
        };
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
                if !is_markdown(to) {
                    tab.document_id = None;
                }
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(tabs: &Tabs) -> Vec<Option<&str>> {
        tabs.tabs()
            .iter()
            .map(|t| t.path.as_deref().and_then(Path::to_str))
            .collect()
    }

    #[test]
    fn open_replaces_active_tab_and_selects_existing() {
        let mut tabs = Tabs::default();
        let first = tabs.active_id();
        tabs.open("a.md".into());
        assert_eq!(tabs.active_id(), first);
        assert_eq!(paths(&tabs), [Some("a.md")]);
        tabs.new_tab();
        tabs.open("b.md".into());
        assert_eq!(paths(&tabs), [Some("a.md"), Some("b.md")]);
        tabs.open("a.md".into());
        assert_eq!(tabs.active_id(), first);
        assert_eq!(tabs.tabs().len(), 2);
    }

    #[test]
    fn close_selects_left_neighbour_and_keeps_one_tab() {
        let mut tabs = Tabs::default();
        tabs.open("a.md".into());
        tabs.new_tab();
        tabs.open("b.md".into());
        tabs.new_tab();
        tabs.open("c.md".into());
        let b = tabs.tabs()[1].id;
        tabs.close(tabs.active_id());
        assert_eq!(tabs.active_id(), b);
        let a = tabs.tabs()[0].id;
        tabs.select(a);
        tabs.close(a);
        assert_eq!(tabs.active_id(), b);
        tabs.close(b);
        assert_eq!(tabs.tabs().len(), 1);
        assert!(tabs.active().path.is_none());
    }

    #[test]
    fn open_new_adds_tab_or_selects_existing() {
        let mut tabs = Tabs::default();
        tabs.open("a.md".into());
        tabs.open_new("b.md".into());
        assert_eq!(paths(&tabs), [Some("a.md"), Some("b.md")]);
        let b = tabs.active_id();
        tabs.open("a.md".into());
        tabs.open_new("b.md".into());
        assert_eq!(tabs.active_id(), b);
        assert_eq!(tabs.tabs().len(), 2);
    }

    #[test]
    fn reorder_moves_tab() {
        let mut tabs = Tabs::default();
        tabs.open("a.md".into());
        tabs.new_tab();
        tabs.open("b.md".into());
        tabs.new_tab();
        tabs.open("c.md".into());
        assert!(tabs.reorder(0, 2));
        assert_eq!(paths(&tabs), [Some("b.md"), Some("c.md"), Some("a.md")]);
        assert!(!tabs.reorder(1, 1));
        assert!(!tabs.reorder(0, 9));
    }

    #[test]
    fn rename_follows_moved_note() {
        let mut tabs = Tabs::default();
        tabs.open("a.md".into());
        tabs.new_tab();
        tabs.open("b.md".into());
        assert!(tabs.rename(Path::new("a.md"), Path::new("c.md")));
        assert_eq!(paths(&tabs), [Some("c.md"), Some("b.md")]);
    }

    #[test]
    fn base_tabs_reuse_replace_close_and_follow_renames_without_documents() {
        let base = Tab::base("board.BASE".into());
        assert!(base.is_base());
        assert!(base.document_id.is_none());
        assert!(!base.graph);
        let mut tabs = Tabs::restore(vec![base], None);
        let first = tabs.active_id();
        tabs.open_new("next.base".into());
        let second = tabs.active_id();
        tabs.open("board.BASE".into());
        assert_eq!(tabs.active_id(), first);
        tabs.open_new("next.base".into());
        assert_eq!(tabs.active_id(), second);
        assert_eq!(tabs.tabs().len(), 2);
        assert!(tabs.rename(Path::new("next.base"), Path::new("moved.base")));
        assert_eq!(tabs.active().path.as_deref(), Some(Path::new("moved.base")));
        tabs.open("note.md".into());
        assert!(!tabs.active().is_base());
        tabs.close(second);
        assert_eq!(tabs.active_id(), first);
        tabs.close(first);
        assert!(tabs.active().path.is_none());
        assert!(tabs.active().document_id.is_none());
    }

    #[test]
    fn rename_retains_only_markdown_document_identity() {
        let mut tabs = Tabs::default();
        tabs.open("note.md".into());
        let id = tabs.active_id();
        let document = Uuid::new_v4();
        tabs.tab_mut(id).unwrap().document_id = Some(document);
        assert!(tabs.rename(Path::new("note.md"), Path::new("renamed.MD")));
        assert_eq!(tabs.active().document_id, Some(document));
        assert!(tabs.rename(Path::new("renamed.MD"), Path::new("board.BASE")));
        assert!(tabs.active().is_base());
        assert_eq!(tabs.active().document_id, None);
        assert!(tabs.rename(Path::new("board.BASE"), Path::new("restored.md")));
        assert!(!tabs.active().is_base());
        assert_eq!(tabs.active_id(), id);
        assert_eq!(tabs.active().document_id, None);
        tabs.tab_mut(id).unwrap().document_id = Some(document);
        assert!(tabs.rename(Path::new("restored.md"), Path::new("file.txt")));
        assert_eq!(tabs.active().document_id, None);
    }
}
