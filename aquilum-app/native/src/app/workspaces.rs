use std::path::{Path, PathBuf};

use aquilum_core::search::paths::same_path;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use uuid::Uuid;

use super::{App, Outcome, Popup};
use crate::i18n::{t, t_with};
use crate::ui::dialog::{ConfirmDialog, DialogAnswer, DialogIds};
use crate::ui::widgets::Pressed;
use crate::ui::workspaces::{self, RowAction, WorkspaceDialog};

pub(super) mod cloud;
mod switch;

pub(super) struct WorkspacesState {
    pub(super) dialog: WorkspaceDialog,
    pub(super) forget: Option<(DialogIds, Uuid)>,
    cloud: Option<cloud::Cloud>,
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
                    if !unique.iter().any(|(_, p)| same_path(Path::new(p), &path)) {
                        unique.push((w.id, path.to_string_lossy().into_owned()));
                    }
                }
                (unique, error)
            }
            Err(e) => {
                report!("список баз: {e:?}");
                (Vec::new(), Some(t_with("workspaces.listFailed", &[("reason", &format!("{e:?}"))])))
            }
        };
        let (layer, dialog) = workspaces::dialog(&known, self.tree.root(), error.as_deref(), self.signed_in());
        let cloud = dialog.cloud.map(|slot| self.list_cloud(slot));
        self.show_layer(root, layer, Point::ORIGIN, |_| Popup::Workspaces(WorkspacesState { dialog, forget: None, cloud }));
    }

    fn close_workspaces(&mut self, root: &mut RenderRoot) {
        if let Some(Popup::Workspaces(state)) = self.popup.take() {
            if let Some((forget, _)) = state.forget {
                crate::ui::dialog::close(root, forget.layer);
            }
            crate::ui::dialog::close(root, state.dialog.layer);
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
            crate::ui::dialog::close(root, forget.layer);
            if answer == DialogAnswer::Cancel {
                root.focus_on(Some(state.dialog.layer));
                self.popup = Some(Popup::Workspaces(state));
                return Outcome::default();
            }
            root.remove_layer(state.dialog.layer);
            let error = self.workspace.core.ui_state.forget_workspace(workspace).err().map(|e| {
                report!("база не забыта: {e:?}");
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
            crate::ui::dialog::close(root, dialog.layer);
            return Outcome::default();
        }
        if id == dialog.open_folder && pressed {
            self.popup = Some(Popup::Workspaces(state));
            return self.pick_workspace(root);
        }
        if let Some((name, sender)) = state.cloud.as_ref().and_then(|cloud| cloud.row(id))
            && matches!(action.downcast_ref::<RowAction>(), Some(RowAction::Open))
        {
            self.popup = Some(Popup::Workspaces(state));
            return self.choose_cloud(root, name, sender);
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
        let sender = self.picked_tx.clone();
        self.pick_folder(move |path| sender.send(super::book::Picked::Workspace(path)).is_ok());
        Outcome::default()
    }

    fn pick_folder(&self, deliver: impl FnOnce(PathBuf) -> bool + Send + 'static) {
        let start = self.tree.root().and_then(Path::parent).map(Path::to_path_buf);
        let wake = std::sync::Arc::clone(&self.workspace.wake);
        std::thread::spawn(move || {
            if let Some(path) = crate::native_dialog::pick_folder(start.as_deref())
                && deliver(path)
            {
                wake();
            }
        });
    }
}
