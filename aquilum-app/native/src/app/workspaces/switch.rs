use std::path::PathBuf;

use aquilum_core::search::paths::canonical_path;
use masonry::app::RenderRoot;

use super::super::{App, Outcome, Popup};
use crate::i18n::t_with;
use crate::session::Session;
use crate::tabs::Tabs;
use crate::ui::file_tree::FileTree;

impl App {
    pub(in crate::app) fn switch_workspace(&mut self, root: &mut RenderRoot, path: PathBuf) -> Outcome {
        let path = canonical_path(&path);
        let opened = path.is_dir() && self.workspace.core.ui_state.resolve_workspace(&path.to_string_lossy(), crate::dates::now_ms()).is_ok();
        if !opened {
            report!("база {} не открыта", path.display());
            let error = t_with("workspaces.openFailed", &[("path", &path.to_string_lossy())]);
            if matches!(self.popup, Some(Popup::Workspaces(_))) || self.tree.root().is_some() {
                self.close_workspaces(root);
                self.show_workspaces(root, Some(error));
            } else {
                self.vault_error = Some(error);
                self.replace_root(root);
                self.show_active(root);
            }
            return Outcome::default();
        }
        self.close_workspaces(root);
        if self.is_current(&path) {
            return Outcome::default();
        }
        self.load_vault(root, Some(path));
        Outcome { title: true, ..Outcome::default() }
    }

    pub(in crate::app) fn load_vault(&mut self, root: &mut RenderRoot, path: Option<PathBuf>) {
        self.leave_note();
        if let Some(session) = &mut self.session {
            session.save(&self.workspace.core, &mut self.tabs);
        }
        self.workspace.root = path.clone();
        self.tree = FileTree::new(path.clone(), &self.workspace.data_dir, std::sync::Arc::clone(&self.workspace.wake));
        self.navigation = crate::navigation::Navigation::load(&self.workspace.data_dir, path.as_deref());
        (self.session, self.tabs) = match path.as_deref().and_then(|path| Session::open(&self.workspace.core, path)) {
            Some((session, tabs)) => (Some(session), tabs),
            None => (None, Tabs::default()),
        };
        self.text = crop::Rope::new();
        self.vault_error = None;
        self.renaming_tab = None;
        self.rename_field = None;
        self.forget(super::super::Moment::Vault, root);
        self.start_index();
        self.refresh_home();
        self.cleanup_retention();
        self.replace_root(root);
        self.show_active(root);
    }
}
