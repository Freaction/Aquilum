use std::path::{Path, PathBuf};

use aquilum_core::search::paths::same_path;
use aquilum_core::ui_state::models::KnownWorkspace;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId};
use masonry::kurbo::Point;
use uuid::Uuid;

use super::book::Picked;
use super::{App, Outcome, Popup};
use crate::i18n::t_with;
use crate::ui::dialog::{DialogAnswer, DialogIds};
use crate::ui::editor::TextAction;
use crate::ui::menu::MenuChoice;
use crate::ui::widgets::{Pressed, Slot};
use crate::ui::workspaces::{self, RowAction, Rows, WorkspaceDialog};

pub(super) mod cloud;
pub(super) mod manage;
mod switch;

pub(super) struct WorkspacesState {
    pub(super) dialog: WorkspaceDialog,
    pub(super) vaults: Vec<KnownWorkspace>,
    pub(super) rows: Rows,
    pub(super) error: Option<String>,
    open: Open,
    cloud: Option<cloud::Cloud>,
}

enum Open {
    List,
    Menu(WidgetId, Uuid),
    Rename(Uuid),
    Delete(DialogIds, Uuid),
}

impl WorkspacesState {
    pub(super) fn cancel(&self) -> (WidgetId, ErasedAction) {
        match (&self.open, self.rows.rename) {
            (Open::Menu(layer, _), _) => (*layer, Box::new(MenuChoice::Dismissed)),
            (Open::Delete(confirm, _), _) => (confirm.layer, Box::new(DialogAnswer::Cancel)),
            (Open::Rename(_), Some(field)) => (field.id(), Box::new(TextAction::Cancelled)),
            _ => (self.dialog.layer, Box::new(DialogAnswer::Cancel)),
        }
    }

    fn known(&self, id: Uuid) -> Option<&KnownWorkspace> {
        self.vaults.iter().find(|vault| vault.id == id)
    }
}

impl App {
    pub(super) fn open_workspaces(&mut self, root: &mut RenderRoot) {
        self.show_workspaces(root, None);
    }

    fn show_workspaces(&mut self, root: &mut RenderRoot, mut error: Option<String>) {
        let (vaults, rows, list) = self.vault_rows(None, &mut error);
        let (layer, dialog) = workspaces::dialog(list, self.signed_in());
        let cloud = dialog.cloud.map(|slot| self.list_cloud(slot));
        self.show_layer(root, layer, Point::ORIGIN, |_| Popup::Workspaces(WorkspacesState { dialog, vaults, rows, error, open: Open::List, cloud }));
    }

    fn vault_rows(&self, renaming: Option<Uuid>, error: &mut Option<String>) -> (Vec<KnownWorkspace>, Rows, NewWidget<dyn Widget>) {
        let vaults = self.workspace.core.ui_state.list_workspaces().unwrap_or_else(|failure| {
            report!("список баз: {failure:?}");
            *error = Some(t_with("workspaces.listFailed", &[("reason", &format!("{failure:?}"))]));
            Vec::new()
        });
        let paths: Vec<&str> = vaults.iter().map(|vault| vault.path.as_str()).collect();
        let current = self.tree.root().and_then(|root| paths.iter().position(|path| same_path(root, Path::new(path))));
        let renaming = renaming.and_then(|id| vaults.iter().position(|vault| vault.id == id));
        let (list, rows) = workspaces::rows(&paths, current, renaming, error.as_deref(), self.compact());
        (vaults, rows, list)
    }

    fn refresh_rows(&mut self, root: &mut RenderRoot, state: &mut WorkspacesState) {
        let renaming = match state.open {
            Open::Rename(id) => Some(id),
            _ => None,
        };
        let (vaults, rows, list) = self.vault_rows(renaming, &mut state.error);
        state.dialog.list.edit(root, |mut slot| Slot::set_child(&mut slot, list));
        (state.vaults, state.rows) = (vaults, rows);
        match state.rows.rename {
            Some(field) => field.focus_all(root),
            None => {
                root.focus_on(Some(state.dialog.layer));
            }
        }
    }

    fn close_workspaces(&mut self, root: &mut RenderRoot) {
        if let Some(Popup::Workspaces(state)) = self.popup.take() {
            match state.open {
                Open::Menu(layer, _) => root.remove_layer(layer),
                Open::Delete(confirm, _) => crate::ui::dialog::close(root, confirm.layer),
                Open::List | Open::Rename(_) => {}
            }
            crate::ui::dialog::close(root, state.dialog.layer);
        }
    }

    fn keep_vaults(&mut self, state: WorkspacesState) -> Outcome {
        self.popup = Some(Popup::Workspaces(state));
        Outcome::default()
    }

    pub(super) fn on_workspaces_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Workspaces(mut state)) = self.popup.take() else { return Outcome::default() };
        match std::mem::replace(&mut state.open, Open::List) {
            Open::Menu(layer, vault) => {
                match action.downcast_ref::<MenuChoice>() {
                    Some(MenuChoice::Item(item)) => {
                        root.remove_layer(layer);
                        return self.vault_command(root, state, vault, *item);
                    }
                    Some(MenuChoice::Dismissed) => self.refresh_rows(root, &mut state),
                    _ => state.open = Open::Menu(layer, vault),
                }
                return self.keep_vaults(state);
            }
            Open::Delete(confirm, vault) => {
                match confirm.answer(id, action) {
                    Some(answer) => {
                        crate::ui::dialog::close(root, confirm.layer);
                        if answer == DialogAnswer::Confirm {
                            self.delete_vault(root, &mut state, vault);
                        }
                        self.refresh_rows(root, &mut state);
                    }
                    None => state.open = Open::Delete(confirm, vault),
                }
                return self.keep_vaults(state);
            }
            Open::Rename(vault) => {
                let field = state.rows.rename;
                let typed = field.is_some_and(|field| field.id() == id);
                let clicked = action.downcast_ref::<Pressed>().is_some() || action.downcast_ref::<RowAction>().is_some() || action.downcast_ref::<DialogAnswer>().is_some();
                match action.downcast_ref::<TextAction>() {
                    Some(TextAction::Entered(name)) if typed => self.rename_vault(root, &mut state, vault, name),
                    Some(TextAction::Cancelled) if typed => self.refresh_rows(root, &mut state),
                    None if clicked => {
                        let name = field.and_then(|field| field.get(root)).map(|field| field.inner().text());
                        if let Some(name) = name {
                            self.rename_vault(root, &mut state, vault, &name);
                        }
                    }
                    _ => state.open = Open::Rename(vault),
                }
                return self.keep_vaults(state);
            }
            Open::List => {}
        }
        self.on_list_action(root, state, id, action)
    }

    fn on_list_action(&mut self, root: &mut RenderRoot, mut state: WorkspacesState, id: WidgetId, action: &ErasedAction) -> Outcome {
        let dialog = &state.dialog;
        let pressed = action.downcast_ref::<Pressed>().is_some();
        if (id == dialog.layer && matches!(action.downcast_ref::<DialogAnswer>(), Some(DialogAnswer::Cancel))) || (id == dialog.close && pressed) {
            crate::ui::dialog::close(root, dialog.layer);
            return Outcome::default();
        }
        if pressed && id == dialog.buttons.create {
            self.create_vault();
        } else if pressed && Some(id) == dialog.buttons.open {
            self.pick_workspace();
        }
        let row = action.downcast_ref::<RowAction>();
        if let Some(name) = state.cloud.as_ref().and_then(|cloud| cloud.row(id))
            && matches!(row, Some(RowAction::Open))
        {
            self.pick_folder(crate::system::vaults_home(), move |parent| Picked::CloudVault(parent.join(name)));
        }
        let vault = state.rows.ids.iter().position(|row| *row == id).and_then(|index| state.vaults.get(index)).map(|vault| (vault.id, PathBuf::from(&vault.path)));
        match (vault, row) {
            (Some((_, path)), Some(RowAction::Open)) => {
                self.popup = Some(Popup::Workspaces(state));
                return self.switch_workspace(root, path);
            }
            (Some((vault, _)), Some(RowAction::Menu(at))) => self.open_vault_menu(root, &mut state, vault, *at),
            _ => {}
        }
        self.keep_vaults(state)
    }
}
