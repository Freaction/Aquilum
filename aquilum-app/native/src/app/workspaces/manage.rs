use std::path::{Path, PathBuf};
use std::sync::Arc;

use aquilum_core::search::paths::{canonical_path, same_path, strip_root};
use aquilum_core::settings::models::SyncSettings;
use masonry::app::RenderRoot;
use masonry::kurbo::Point;
use uuid::Uuid;

use super::{App, Open, Outcome, Picked, Popup, WorkspacesState};
use crate::file_ops;
use crate::i18n::{t, t_with};
use crate::ui::dialog::ConfirmDialog;
use crate::ui::icons;
use crate::ui::menu::{Entry, Menu};
use crate::ui::settings::schema::Section;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum Command {
    Rename,
    Duplicate,
    Sync,
    Forget,
    Delete,
}

pub(in crate::app) const COMMANDS: &[Command] = if crate::system::DESKTOP {
    &[Command::Rename, Command::Duplicate, Command::Sync, Command::Forget, Command::Delete]
} else {
    &[Command::Rename, Command::Duplicate, Command::Sync, Command::Delete]
};

impl App {
    pub(super) fn open_vault_menu(&mut self, root: &mut RenderRoot, state: &mut WorkspacesState, vault: Uuid, at: Point) {
        let Some(synced) = state.known(vault).map(|known| known.sync.enabled) else { return };
        let items: Vec<_> = COMMANDS
            .iter()
            .map(|command| match command {
                Command::Rename => (t("common.rename"), icons::PENCIL, synced),
                Command::Duplicate => (t("common.duplicate"), icons::COPY, false),
                Command::Sync if synced => (t("workspaces.syncOff"), icons::CLOUD_OFF, false),
                Command::Sync => (t("workspaces.syncOn"), icons::CLOUD, false),
                Command::Forget => (t("workspaces.forget"), icons::X, false),
                Command::Delete => (t("common.delete"), icons::TRASH, false),
            })
            .collect();
        let entries: Vec<Entry<'_>> = items
            .iter()
            .zip(COMMANDS)
            .map(|((label, icon, disabled), command)| Entry { label, icon: Some(*icon), disabled: *disabled, divided: *command == Command::Delete })
            .collect();
        let menu = Menu::entries(&t("workspaces.actions"), &entries);
        let layer = menu.id();
        root.add_layer(menu, Point::ZERO);
        self.place_layer(root, layer, masonry::kurbo::Rect::from_points(at, at));
        root.focus_on(Some(layer));
        state.open = Open::Menu(layer, vault);
    }

    pub(super) fn vault_command(&mut self, root: &mut RenderRoot, mut state: WorkspacesState, vault: Uuid, item: usize) -> Outcome {
        state.error = None;
        if let Some((path, synced)) = state.known(vault).map(|known| (PathBuf::from(&known.path), known.sync.enabled))
            && let Some(command) = COMMANDS.get(item)
        {
            match command {
                Command::Rename => state.open = Open::Rename(vault),
                Command::Duplicate => {
                    let core = Arc::clone(&self.workspace.core);
                    let copy = crate::app::jobs::deliver("копия базы", self.picked_tx.clone(), &self.workspace.wake, move || Some(Picked::NewVault(file_ops::duplicate(&core, &path).map_err(|error| error.to_string()))));
                    self.own(copy);
                }
                Command::Sync => {
                    self.update_sync(&path, |sync| sync.enabled = !synced);
                    if !synced && !self.signed_in() {
                        crate::ui::dialog::close(root, state.dialog.layer);
                        self.show_settings(root, Section::Sync);
                        return Outcome::default();
                    }
                }
                Command::Forget => {
                    self.forget_vault(root, &mut state, vault, "workspaces.forgetFailed");
                }
                Command::Delete => {
                    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
                    let description = t_with("workspaces.deleteDescription", &[("name", &name)]);
                    let (layer, confirm) = ConfirmDialog::new(&t("workspaces.deleteTitle"), &description, &t("common.cancel"), &t("common.delete"));
                    root.add_layer(layer, Point::ORIGIN);
                    root.focus_on(Some(confirm.layer));
                    state.open = Open::Delete(confirm, vault);
                    return self.keep_vaults(state);
                }
            }
        }
        self.refresh_rows(root, &mut state);
        self.keep_vaults(state)
    }

    pub(super) fn rename_vault(&mut self, root: &mut RenderRoot, state: &mut WorkspacesState, vault: Uuid, input: &str) {
        if let Some(from) = state.known(vault).map(|known| PathBuf::from(&known.path))
            && let Some((to, name)) = file_ops::rename_target(&from, input)
        {
            state.error = self.move_vault(root, vault, &from, &to).err().map(|reason| {
                report!("база {} не переименована: {reason}", from.display());
                t_with("workspaces.renameFailed", &[("name", &name), ("reason", &reason)])
            });
        }
        self.refresh_rows(root, state);
    }

    fn move_vault(&mut self, root: &mut RenderRoot, vault: Uuid, from: &Path, to: &Path) -> Result<(), String> {
        if to.exists() && !same_path(from, to) {
            return Err(t("workspaces.nameTaken"));
        }
        let open = self.open_under(from);
        if open.is_some() {
            self.leave_note();
            crate::disk::wait();
        }
        let moved = std::fs::rename(from, to).map_err(|error| error.to_string()).and_then(|()| {
            self.workspace.core.ui_state.move_workspace(vault, to).map_err(|error| {
                let _ = std::fs::rename(to, from);
                format!("{error:?}")
            })
        });
        match (open, &moved) {
            (Some(rest), Ok(())) => self.load_vault(root, Some(canonical_path(&to.join(rest)))),
            (Some(_), Err(_)) => self.show_active(root),
            (None, _) => {}
        }
        moved
    }

    pub(super) fn delete_vault(&mut self, root: &mut RenderRoot, state: &mut WorkspacesState, vault: Uuid) {
        let Some(path) = self.forget_vault(root, state, vault, "workspaces.deleteFailed") else { return };
        crate::disk::run(move || {
            if let Err(error) = std::fs::remove_dir_all(&path) {
                report!("папка базы {} не удалена: {error}", path.display());
            }
        });
    }

    fn forget_vault(&mut self, root: &mut RenderRoot, state: &mut WorkspacesState, vault: Uuid, failed: &str) -> Option<PathBuf> {
        let path = state.known(vault).map(|known| PathBuf::from(&known.path))?;
        if self.open_under(&path).is_some() {
            let next = state.vaults.iter().map(|known| PathBuf::from(&known.path)).find(|other| strip_root(&path, other).is_none() && other.is_dir());
            self.load_vault(root, next);
        }
        match self.workspace.core.ui_state.forget_workspace(vault) {
            Ok(()) => Some(path),
            Err(error) => {
                report!("база {} не убрана из списка: {error:?}", path.display());
                state.error = Some(t(failed));
                None
            }
        }
    }

    pub(in crate::app) fn create_vault(&self) {
        let name = t("workspaces.newName");
        self.pick_folder(crate::system::vaults_home(), move |parent| Picked::NewVault(file_ops::create_folder(&parent, &name, 1).map_err(|error| error.to_string())));
    }

    pub(in crate::app) fn pick_workspace(&self) {
        self.pick_folder(None, Picked::Workspace);
    }

    pub(super) fn pick_folder(&self, home: Option<PathBuf>, then: impl FnOnce(PathBuf) -> Picked + Send + 'static) {
        let start = self.tree.root().and_then(Path::parent).map(Path::to_path_buf);
        self.pick(move || home.or_else(|| crate::native_dialog::pick_folder(start.as_deref())).map(then));
    }

    pub(in crate::app) fn vault_made(&mut self, root: &mut RenderRoot, made: Result<PathBuf, String>) -> bool {
        let path = match made {
            Ok(path) => canonical_path(&path),
            Err(reason) => {
                report!("база не создана: {reason}");
                if self.popup.is_none() || matches!(self.popup, Some(Popup::Workspaces(_))) {
                    self.close_workspaces(root);
                    self.show_workspaces(root, Some(t_with("workspaces.createFailed", &[("reason", &reason)])));
                }
                return false;
            }
        };
        if self.tree.root().is_none() {
            return self.switch_workspace(root, path).title;
        }
        let remembered = self.workspace.core.ui_state.resolve_workspace(&path.to_string_lossy(), crate::dates::now_ms());
        if let Some(Popup::Workspaces(mut state)) = self.popup.take_if(|popup| matches!(popup, Popup::Workspaces(state) if matches!(state.open, Open::List))) {
            match remembered {
                Ok(id) => state.open = Open::Rename(id),
                Err(error) => state.error = Some(t_with("workspaces.listFailed", &[("reason", &format!("{error:?}"))])),
            }
            self.refresh_rows(root, &mut state);
            self.popup = Some(Popup::Workspaces(state));
        }
        false
    }

    pub(in crate::app) fn update_sync(&self, path: &Path, set: impl FnOnce(&mut SyncSettings)) {
        let core = &self.workspace.core;
        let mut sync = core.ui_state.workspace(path).map(|known| known.sync).unwrap_or_default();
        set(&mut sync);
        if let Err(error) = core.ui_state.set_sync(&path.to_string_lossy(), &sync) {
            report!("настройки синхронизации базы не сохранены: {error:?}");
        }
        if self.is_current(path) {
            self.bind_sync();
        }
    }

    pub(super) fn is_current(&self, path: &Path) -> bool {
        self.tree.root().is_some_and(|root| same_path(root, path))
    }

    fn open_under(&self, path: &Path) -> Option<PathBuf> {
        self.tree.root().and_then(|root| strip_root(path, root)).map(Path::to_path_buf)
    }
}
