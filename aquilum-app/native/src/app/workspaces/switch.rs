use std::path::PathBuf;

use aquilum_core::search::paths::same_path;
use masonry::app::RenderRoot;

use super::super::{App, Outcome, Popup};
use crate::i18n::t_with;
use crate::session::Session;
use crate::tabs::Tabs;
use crate::ui::file_tree::FileTree;

impl App {
    pub(in crate::app) fn switch_workspace(&mut self, root: &mut RenderRoot, path: PathBuf) -> Outcome {
        let path = crate::workspace::normalize(&path);
        let reopen = matches!(self.popup, Some(Popup::Workspaces(_)));
        let failed = |app: &mut App, root: &mut RenderRoot| {
            let error = t_with("workspaces.openFailed", &[("path", &path.to_string_lossy())]);
            if reopen || app.tree.root().is_some() {
                app.close_workspaces(root);
                app.show_workspaces(root, Some(error));
            } else {
                app.vault_error = Some(error);
                app.replace_root(root);
                app.show_active(root);
            }
            Outcome::default()
        };
        if !path.is_dir() {
            return failed(self, root);
        }
        if let Err(e) = self.workspace.core.ui_state.resolve_workspace(&path.to_string_lossy(), crate::dates::now_ms()) {
            report!("база {} не открыта: {e:?}", path.display());
            return failed(self, root);
        }
        self.close_workspaces(root);
        if self.tree.root().is_some_and(|current| same_path(current, &path)) {
            return Outcome::default();
        }
        self.leave_note();
        if let Some(session) = &mut self.session {
            session.save(&self.workspace.core, &mut self.tabs);
        }
        self.workspace.root = Some(path.clone());
        self.tree = FileTree::new(Some(path.clone()), &self.workspace.data_dir, std::sync::Arc::clone(&self.workspace.wake));
        self.navigation = crate::navigation::Navigation::load(&self.workspace.data_dir, Some(&path));
        (self.session, self.tabs) = match Session::open(&self.workspace.core, &path) {
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
        Outcome { title: true, ..Outcome::default() }
    }
}
