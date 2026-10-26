use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use aquilum_core::search::paths::same_path;
use aquilum_core::sync::SyncStatus;
use masonry::app::RenderRoot;
use masonry::core::WidgetId;

use super::{App, Outcome, Popup, WorkspacesState};
use crate::ui::handle::Handle;
use crate::ui::settings::schema::{Setter, Value};
use crate::ui::widgets::Slot;
use crate::ui::workspaces::{Cloud as View, cloud_section};

pub(in crate::app) enum CloudEvent {
    Listed(Result<Vec<String>, String>),
    Chosen(PathBuf),
}

pub(in crate::app) struct Cloud {
    slot: Handle<Slot>,
    events: (Sender<CloudEvent>, Receiver<CloudEvent>),
    rows: Vec<(WidgetId, String)>,
}

impl Cloud {
    pub(super) fn row(&self, id: WidgetId) -> Option<(String, Sender<CloudEvent>)> {
        self.rows.iter().find(|(row, _)| *row == id).map(|(_, name)| (name.clone(), self.events.0.clone()))
    }

    fn show(&mut self, root: &mut RenderRoot, listed: Result<Vec<String>, String>, local: &[(WidgetId, PathBuf, uuid::Uuid)]) {
        let here: Vec<String> = local.iter().filter_map(|(_, path, _)| path.file_name()).map(|name| name.to_string_lossy().to_lowercase()).collect();
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
        let events = channel();
        let (sender, core, wake) = (events.0.clone(), Arc::clone(&self.workspace.core), Arc::clone(&self.workspace.wake));
        std::thread::spawn(move || {
            let listed = core.sync.vaults().map_err(|error| error.to_string());
            if sender.send(CloudEvent::Listed(listed)).is_ok() {
                wake();
            }
        });
        Cloud { slot, events, rows: Vec::new() }
    }

    pub(super) fn choose_cloud(&mut self, root: &mut RenderRoot, name: String, sender: Sender<CloudEvent>) -> Outcome {
        if let Some(home) = crate::system::vaults_home() {
            return self.join_cloud(root, home.join(name));
        }
        self.pick_folder(move |parent| sender.send(CloudEvent::Chosen(parent.join(name))).is_ok());
        Outcome::default()
    }

    pub(in crate::app) fn on_cloud_events(&mut self, root: &mut RenderRoot) -> bool {
        let mut title = false;
        while let Some(Popup::Workspaces(WorkspacesState { cloud: Some(cloud), dialog, .. })) = &mut self.popup
            && let Ok(event) = cloud.events.1.try_recv()
        {
            match event {
                CloudEvent::Listed(listed) => cloud.show(root, listed, &dialog.rows),
                CloudEvent::Chosen(path) => title |= self.join_cloud(root, path).title,
            }
        }
        title
    }

    fn join_cloud(&mut self, root: &mut RenderRoot, path: PathBuf) -> Outcome {
        if let Err(error) = std::fs::create_dir_all(&path) {
            report!("папка базы {} не создана: {error}", path.display());
        }
        let outcome = self.switch_workspace(root, path.clone());
        if self.tree.root().is_some_and(|current| same_path(current, &path)) {
            self.apply(Setter::VaultSync(|sync, _| sync.enabled = true), Value::Bool(true));
        }
        outcome
    }
}

#[cfg(test)]
impl WorkspacesState {
    pub(in crate::app) fn cloud_rows(&self) -> Vec<(WidgetId, String)> {
        self.cloud.as_ref().map_or_else(Vec::new, |cloud| cloud.rows.clone())
    }

    pub(in crate::app) fn cloud_events(&self) -> Option<Sender<CloudEvent>> {
        self.cloud.as_ref().map(|cloud| cloud.events.0.clone())
    }
}
