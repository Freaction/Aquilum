use crate::files::gate::{NoteHistoryChanged, Relocation};
use crate::files::watcher::{WatchBatch, WatchScope, WatchSink, WorkspaceWatcher};
use crate::history::HistoryService;
use crate::mcp::active::ActiveNote;
use crate::mcp::bridge::Navigation;
use crate::mcp::{McpServer, McpStatus};
use crate::search::paths::cache_key;
use crate::search::{IndexNotifier, IndexRevision, SearchService};
use crate::settings::SettingsManager;
use crate::sync::SyncService;
use crate::ui_state::UiStateService;
use crate::wikixiv::WikixivService;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::Duration;

const SYNC_AFTER_EDIT: Duration = Duration::from_secs(3);

/// What the core tells its host.
#[derive(Clone)]
pub enum CoreEvent {
    NotesRelocated(Relocation),
    NoteHistoryChanged(NoteHistoryChanged),
    WorkspaceChanged(Vec<String>),
    ContentChanged(Vec<String>),
    WorkspaceRescanned,
    LinksChanged(IndexRevision),
    Navigate(Navigation),
    SyncChanged,
    Settle(std::sync::mpsc::SyncSender<()>),
}

/// Receives core events. Implementations must not block and must not call back into the core
/// synchronously: events are sent from the core's own threads, sometimes while it holds locks.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: CoreEvent);
}

impl<F: Fn(CoreEvent) + Send + Sync> EventSink for F {
    fn emit(&self, event: CoreEvent) {
        self(event)
    }
}

/// A background task of the host died before finishing (panic or shutdown).
#[derive(Debug)]
pub struct TaskFailed(pub String);

impl std::fmt::Display for TaskFailed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Every service of the app, wired together, with no knowledge of the UI that hosts it.
pub struct Core {
    pub settings: SettingsManager,
    pub ui_state: UiStateService,
    pub search: SearchService,
    pub history: HistoryService,
    pub wikixiv: WikixivService,
    pub watcher: WorkspaceWatcher,
    /// The note open in the UI, reported by the host; MCP tools use it for «эта заметка».
    pub active_note: ActiveNote,
    pub mcp: McpServer,
    pub sync: SyncService,
    events: Arc<dyn EventSink>,
}

impl Core {
    /// Opens all stores under `app_data_dir`, which the host picks.
    pub fn open(app_data_dir: &Path, events: Arc<dyn EventSink>) -> Arc<Self> {
        let core = Arc::new_cyclic(|this: &Weak<Core>| {
            let indexed = Arc::clone(&events);
            let index_notifier: IndexNotifier = Arc::new(move |revision| {
                indexed.emit(CoreEvent::LinksChanged(revision));
            });
            let watched = Weak::clone(this);
            let watch_sink: WatchSink = Arc::new(move |batch| {
                if let Some(core) = watched.upgrade() {
                    core.ingest_watch(batch);
                }
            });
            Core {
                settings: SettingsManager::new(app_data_dir),
                ui_state: UiStateService::open(&app_data_dir.join("ui-state.sqlite3")),
                search: SearchService::new(app_data_dir.join("search-v2"), index_notifier),
                history: HistoryService::new(app_data_dir),
                wikixiv: WikixivService::new(),
                watcher: WorkspaceWatcher::new(watch_sink),
                active_note: ActiveNote::default(),
                mcp: McpServer::new(),
                sync: SyncService::new(app_data_dir),
                events,
            }
        });
        core.sync.start(Arc::downgrade(&core));
        core
    }

    pub fn emit(&self, event: CoreEvent) {
        self.events.emit(event);
    }

    pub(crate) fn synced(&self, paths: Vec<std::path::PathBuf>) {
        if !paths.is_empty() {
            self.ingest_watch(WatchBatch::new(paths, WatchScope::Structure));
        }
    }

    pub fn settle(&self) {
        let (done, settled) = std::sync::mpsc::sync_channel(1);
        self.emit(CoreEvent::Settle(done));
        let _ = settled.recv_timeout(std::time::Duration::from_secs(2));
    }

    /// Starts, restarts or stops the MCP server to match the saved settings.
    pub fn apply_mcp_settings(self: &Arc<Self>) -> McpStatus {
        self.mcp.apply(self, &self.settings.get_config().mcp)
    }

    pub fn shutdown(&self) {
        self.mcp.shutdown();
        self.watcher.close();
        self.sync.close();
        self.search.close();
        self.ui_state.close();
    }

    pub fn known_roots(&self) -> Vec<PathBuf> {
        self.ui_state.workspace_roots().unwrap_or_default()
    }

    pub fn prune_caches(&self) {
        let Ok(roots) = self.ui_state.workspace_roots() else { return };
        let keep = roots.iter().map(|root| cache_key(root)).collect::<HashSet<_>>();
        self.search.prune(&keep);
        self.sync.prune(&keep);
    }

    fn ingest_watch(&self, batch: WatchBatch) {
        let rescan = batch.scope == WatchScope::Rescan;
        if self.sync.touched(&batch.paths, rescan) {
            self.sync.request(SYNC_AFTER_EDIT);
            self.emit(CoreEvent::SyncChanged);
        }
        match batch.scope {
            WatchScope::Content => self.emit(CoreEvent::ContentChanged(path_names(&batch.paths))),
            WatchScope::Rescan => self.emit(CoreEvent::WorkspaceRescanned),
            WatchScope::Listing | WatchScope::Structure => self.emit(CoreEvent::WorkspaceChanged(path_names(&batch.paths))),
        }
        self.search.queue(&batch.paths, rescan);
    }
}

pub(crate) fn path_names(paths: &[PathBuf]) -> Vec<String> {
    paths.iter().map(|path| path.to_string_lossy().into_owned()).collect()
}

#[cfg(test)]
#[path = "app_core_tests.rs"]
mod tests;
