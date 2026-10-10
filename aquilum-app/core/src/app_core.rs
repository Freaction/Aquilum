use crate::documents::{DocumentEvent, DocumentHub, SaveFailed};
use crate::files::gate::{NoteHistoryChanged, Relocation};
use crate::files::watcher::{WatchBatch, WatchScope, WatchSink, WorkspaceWatcher};
use crate::history::HistoryService;
use crate::mcp::active::ActiveNote;
use crate::mcp::bridge::Navigation;
use crate::mcp::{McpServer, McpStatus};
use crate::search::{ChangeNotifier, IndexNotifier, IndexRevision, SearchService};
use crate::settings::SettingsManager;
use crate::ui_state::UiStateService;
use crate::wikixiv::WikixivService;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum CoreEvent {
    DocumentChanged(DocumentEvent),
    DocumentSaved(DocumentEvent),
    DocumentMissing(DocumentEvent),
    DocumentSaveFailed(SaveFailed),
    NotesRelocated(Relocation),
    NoteHistoryChanged(NoteHistoryChanged),
    WorkspaceChanged(Vec<String>),
    LinksChanged(IndexRevision),
    Navigate(Navigation),
}

impl CoreEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DocumentChanged(_) => "document-changed",
            Self::DocumentSaved(_) => "document-saved",
            Self::DocumentMissing(_) => "document-missing",
            Self::DocumentSaveFailed(_) => "document-save-failed",
            Self::NotesRelocated(_) => "notes-relocated",
            Self::NoteHistoryChanged(_) => "note-history-changed",
            Self::WorkspaceChanged(_) => "workspace-changed",
            Self::LinksChanged(_) => "links-changed",
            Self::Navigate(_) => "mcp-navigate",
        }
    }
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: CoreEvent);
}

impl<F: Fn(CoreEvent) + Send + Sync> EventSink for F {
    fn emit(&self, event: CoreEvent) {
        self(event)
    }
}

#[derive(Debug)]
pub struct TaskFailed(pub String);

impl std::fmt::Display for TaskFailed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub struct Core {
    pub settings: SettingsManager,
    pub ui_state: UiStateService,
    pub search: SearchService,
    pub history: HistoryService,
    pub documents: DocumentHub,
    pub wikixiv: WikixivService,
    pub watcher: WorkspaceWatcher,
    pub active_note: ActiveNote,
    pub mcp: McpServer,
    events: Arc<dyn EventSink>,
}

impl Core {
    pub fn open(app_data_dir: &Path, events: Arc<dyn EventSink>) -> Arc<Self> {
        let core = Arc::new_cyclic(|this: &Weak<Core>| {
            let changed = Arc::clone(&events);
            let notifier: ChangeNotifier = Arc::new(move |paths| {
                changed.emit(CoreEvent::WorkspaceChanged(path_names(&paths)));
            });
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
                search: SearchService::new(
                    app_data_dir.join("search-v2"),
                    notifier,
                    index_notifier,
                ),
                history: HistoryService::new(app_data_dir),
                documents: DocumentHub::new(&app_data_dir.join("documents.sqlite3")),
                wikixiv: WikixivService::new(),
                watcher: WorkspaceWatcher::new(watch_sink),
                active_note: ActiveNote::default(),
                mcp: McpServer::new(),
                events,
            }
        });
        core.documents.start(Arc::downgrade(&core));
        core
    }

    pub fn emit(&self, event: CoreEvent) {
        self.events.emit(event);
    }

    pub fn apply_mcp_settings(self: &Arc<Self>) -> McpStatus {
        self.mcp.apply(self, &self.settings.get_config().mcp)
    }

    pub fn shutdown(&self) {
        self.documents.flush_all(self);
        self.mcp.shutdown();
        self.watcher.stop();
    }

    pub fn known_roots(&self) -> Vec<PathBuf> {
        self.ui_state.workspace_roots().unwrap_or_default()
    }

    fn ingest_watch(&self, batch: WatchBatch) {
        if batch.scope >= WatchScope::Structure
            || batch.paths.iter().any(|path| {
                path.extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("base"))
            })
        {
            self.emit(CoreEvent::WorkspaceChanged(path_names(&batch.paths)));
        }
        self.documents.reconcile_paths(self, &batch.paths);
        self.search
            .ingest_watch(batch.paths, batch.scope == WatchScope::Rescan);
    }
}

fn path_names(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
#[path = "app_core_tests.rs"]
mod tests;
