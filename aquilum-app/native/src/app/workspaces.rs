use std::collections::HashMap;
use std::path::{Path, PathBuf};

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use uuid::Uuid;

use super::{App, Outcome, Popup};
use crate::i18n::{t, t_with};
use crate::session::Session;
use crate::tabs::Tabs;
use crate::ui::dialog::{ConfirmDialog, DialogAnswer, DialogIds};
use crate::ui::file_tree::FileTree;
use crate::ui::widgets::Pressed;
use crate::ui::workspaces::{self, RowAction, WorkspaceDialog};

pub(super) struct WorkspacesState {
    dialog: WorkspaceDialog,
    forget: Option<(DialogIds, Uuid)>,
}

#[cfg(test)]
impl WorkspacesState {
    pub(super) fn dialog(&self) -> &WorkspaceDialog {
        &self.dialog
    }

    pub(super) fn forget_layer(&self) -> Option<WidgetId> {
        self.forget.as_ref().map(|(ids, _)| ids.layer)
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

impl App {
    pub(super) fn open_workspaces(&mut self, root: &mut RenderRoot) {
        self.show_workspaces(root, None);
    }

    fn show_workspaces(&mut self, root: &mut RenderRoot, error: Option<String>) {
        let (known, error) = match self.workspace.core.ui_state.list_workspaces() {
            Ok(known) => {
                let mut unique: Vec<(uuid::Uuid, String)> = Vec::new();
                for w in known {
                    let path = crate::workspace::normalize(std::path::Path::new(&w.path));
                    if !unique.iter().any(|(_, p)| workspaces::same_path(std::path::Path::new(p), &path)) {
                        unique.push((w.id, path.to_string_lossy().into_owned()));
                    }
                }
                (unique, error)
            }
            Err(e) => {
                eprintln!("список баз: {e:?}");
                (Vec::new(), Some(t_with("workspaces.listFailed", &[("reason", &format!("{e:?}"))])))
            }
        };
        let (layer, dialog) = workspaces::dialog(&known, self.tree.root(), error.as_deref());
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(dialog.layer));
        self.popup = Some(Popup::Workspaces(WorkspacesState { dialog, forget: None }));
    }

    fn close_workspaces(&mut self, root: &mut RenderRoot) {
        if let Some(Popup::Workspaces(state)) = self.popup.take() {
            if let Some((forget, _)) = state.forget {
                root.remove_layer(forget.layer);
            }
            root.remove_layer(state.dialog.layer);
        }
    }

    pub(super) fn on_workspaces_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Workspaces(mut state)) = self.popup.take() else { return Outcome::default() };
        if let Some((forget, workspace)) = state.forget.take() {
            let Some(answer) = forget.answer(id, action) else {
                state.forget = Some((forget, workspace));
                self.popup = Some(Popup::Workspaces(state));
                return Outcome::default();
            };
            root.remove_layer(forget.layer);
            if answer == DialogAnswer::Cancel {
                root.focus_on(Some(state.dialog.layer));
                self.popup = Some(Popup::Workspaces(state));
                return Outcome::default();
            }
            root.remove_layer(state.dialog.layer);
            let error = self.workspace.core.ui_state.forget_workspace(workspace).err().map(|e| {
                eprintln!("база не забыта: {e:?}");
                t("workspaces.forgetFailed")
            });
            self.show_workspaces(root, error);
            return Outcome::default();
        }
        let dialog = &state.dialog;
        let pressed = action.downcast_ref::<Pressed>().is_some();
        let closing = (id == dialog.layer && matches!(action.downcast_ref::<DialogAnswer>(), Some(DialogAnswer::Cancel)))
            || (id == dialog.close && pressed);
        if closing {
            root.remove_layer(dialog.layer);
            return Outcome::default();
        }
        if id == dialog.open_folder && pressed {
            self.popup = Some(Popup::Workspaces(state));
            return self.pick_workspace(root);
        }
        let row = dialog.rows.iter().find(|(row, _, _)| *row == id).map(|(_, path, uuid)| (path.clone(), *uuid));
        match (row, action.downcast_ref::<RowAction>()) {
            (Some((path, _)), Some(RowAction::Open)) => {
                self.popup = Some(Popup::Workspaces(state));
                return self.switch_workspace(root, path);
            }
            (Some((path, workspace)), Some(RowAction::Forget)) => {
                let (title, description) = workspaces::forget_texts(&path);
                let (layer, forget) = ConfirmDialog::new(&title, &description, &t("common.cancel"), &t("workspaces.forgetConfirm"));
                root.add_layer(layer, Point::ORIGIN);
                root.focus_on(Some(forget.layer));
                state.forget = Some((forget, workspace));
            }
            _ => {}
        }
        self.popup = Some(Popup::Workspaces(state));
        Outcome::default()
    }

    pub(super) fn pick_workspace(&mut self, _root: &mut RenderRoot) -> Outcome {
        let start = self.tree.root().and_then(Path::parent).map(Path::to_path_buf);
        let sender = self.picked_tx.clone();
        let wake = std::sync::Arc::clone(&self.workspace.wake);
        std::thread::spawn(move || {
            let mut picker = rfd::FileDialog::new();
            if let Some(start) = start {
                picker = picker.set_directory(start);
            }
            if let Some(path) = picker.pick_folder()
                && sender.send(super::book::Picked::Workspace(path)).is_ok()
            {
                wake();
            }
        });
        Outcome::default()
    }

    pub(super) fn switch_workspace(&mut self, root: &mut RenderRoot, path: PathBuf) -> Outcome {
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
        if let Err(e) = self.workspace.core.ui_state.resolve_workspace(&path.to_string_lossy(), now_ms()) {
            eprintln!("база {} не открыта: {e:?}", path.display());
            return failed(self, root);
        }
        self.close_workspaces(root);
        if self.tree.root().is_some_and(|current| workspaces::same_path(current, &path)) {
            return Outcome::default();
        }
        self.invalidate_kanban();
        self.leave_note();
        if let Some(session) = &mut self.session {
            session.save(&self.workspace.core, &mut self.tabs);
        }
        self.workspace.root = Some(path.clone());
        self.tree = FileTree::new(Some(path.clone()), &self.workspace.data_dir);
        self.navigation = crate::navigation::Navigation::load(&self.workspace.data_dir, Some(&path));
        (self.session, self.tabs) = match Session::open(&self.workspace.core, &path) {
            Some((session, tabs)) => (Some(session), tabs),
            None => (None, Tabs::default()),
        };
        self.editor_text.clear();
        self.vault_error = None;
        self.renaming_tab = None;
        self.rename_field = None;
        self.listing = None;
        self.panel_body = None;
        self.history = None;
        self.history_body = None;
        self.opened_versions = HashMap::new();
        self.version_view = None;
        self.version_notice = None;
        self.home = None;
        self.start_index();
        self.refresh_home();
        self.replace_root(root);
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }
}
