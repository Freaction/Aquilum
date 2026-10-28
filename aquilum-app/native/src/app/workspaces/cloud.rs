use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use aquilum_core::sync::SyncStatus;
use aquilum_core::ui_state::models::KnownWorkspace;
use masonry::app::RenderRoot;
use masonry::core::WidgetId;

use super::{App, Outcome, Popup, WorkspacesState};
use crate::ui::handle::Handle;
use crate::ui::widgets::Slot;
use crate::ui::workspaces::{Cloud as View, cloud_section};

pub(in crate::app) type Listed = Result<Vec<String>, String>;

pub(in crate::app) struct Cloud {
    slot: Handle<Slot>,
    listed: (Sender<Listed>, Receiver<Listed>),
    rows: Vec<(WidgetId, String)>,
}

impl Cloud {
    pub(super) fn row(&self, id: WidgetId) -> Option<String> {
        self.rows.iter().find(|(row, _)| *row == id).map(|(_, name)| name.clone())
    }

    fn show(&mut self, root: &mut RenderRoot, listed: Listed, local: &[KnownWorkspace]) {
        let here: Vec<String> = local.iter().filter_map(|known| Path::new(&known.path).file_name()).map(|name| name.to_string_lossy().to_lowercase()).collect();
        let missing = listed.map(|names| names.into_iter().filter(|name| !here.contains(&name.to_lowercase())).collect::<Vec<_>>());
        let (widget, rows) = cloud_section(match &missing {
            Ok(names) => View::Vaults(names),
            Err(error) => View::Failed(error),
        });
        self.slot.edit(root, |mut slot| Slot::set_child(&mut slot, widget));
        self.rows = rows;
    }
}

impl App {
    pub(super) fn signed_in(&self) -> bool {
        !matches!(self.workspace.core.sync.status(), SyncStatus::Off(_) | SyncStatus::Login)
    }

    pub(super) fn list_cloud(&self, slot: Handle<Slot>) -> Cloud {
        let listed = channel();
        let (sender, core, wake) = (listed.0.clone(), Arc::clone(&self.workspace.core), Arc::clone(&self.workspace.wake));
        std::thread::spawn(move || {
            if sender.send(core.sync.vaults().map_err(|error| error.to_string())).is_ok() {
                wake();
            }
        });
        Cloud { slot, listed, rows: Vec::new() }
    }

    pub(in crate::app) fn poll_cloud(&mut self, root: &mut RenderRoot) {
        if let Some(Popup::Workspaces(WorkspacesState { cloud: Some(cloud), vaults, .. })) = &mut self.popup
            && let Ok(listed) = cloud.listed.1.try_recv()
        {
            cloud.show(root, listed, vaults);
        }
    }

    pub(in crate::app) fn join_cloud(&mut self, root: &mut RenderRoot, path: PathBuf) -> Outcome {
        if let Err(error) = std::fs::create_dir_all(&path) {
            report!("папка базы {} не создана: {error}", path.display());
        }
        let outcome = self.switch_workspace(root, path.clone());
        if self.is_current(&path) {
            self.update_sync(&path, |sync| sync.enabled = true);
        }
        outcome
    }
}

#[cfg(test)]
impl WorkspacesState {
    pub(in crate::app) fn cloud_rows(&self) -> Vec<(WidgetId, String)> {
        self.cloud.as_ref().map_or_else(Vec::new, |cloud| cloud.rows.clone())
    }

    pub(in crate::app) fn cloud_listed(&self) -> Option<Sender<Listed>> {
        self.cloud.as_ref().map(|cloud| cloud.listed.0.clone())
    }
}
