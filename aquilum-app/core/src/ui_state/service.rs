use super::database::UiStateDatabase;
use super::error::UiStateError;
use super::models::{KnownWorkspace, LoadedSession, OpenSessionInput, SaveStateBatchInput};
use super::validation::{validate_batch, validate_open_session};
use crate::search::paths::canonical_path;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const PURGE_BATCH: i64 = 5_000;
const VACUUM_PAGES: i64 = 1_000;

#[derive(Clone)]
pub struct UiStateService {
    path: PathBuf,
    storage: Arc<Mutex<Storage>>,
}

enum Storage {
    Ready(UiStateDatabase),
    Unavailable(String),
}

impl UiStateService {
    pub fn open(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            storage: Arc::new(Mutex::new(open_storage(path))),
        }
    }

    pub fn reset(&self, now_ms: i64) -> Result<(), UiStateError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|error| UiStateError::Database {
                message: error.to_string(),
            })?;
        *storage = Storage::Unavailable("сбрасывается".to_owned());
        let set_aside = set_aside_database(&self.path, now_ms);
        *storage = open_storage(&self.path);
        set_aside
    }

    fn with_database<T>(
        &self,
        operation: impl FnOnce(&mut UiStateDatabase) -> Result<T, UiStateError>,
    ) -> Result<T, UiStateError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|error| UiStateError::Database {
                message: error.to_string(),
            })?;
        match &mut *storage {
            Storage::Ready(database) => operation(database),
            Storage::Unavailable(message) => Err(UiStateError::Unavailable {
                message: message.clone(),
            }),
        }
    }

    pub fn list_workspaces(&self) -> Result<Vec<KnownWorkspace>, UiStateError> {
        self.with_database(|database| database.list_workspaces())
    }

    pub fn workspace_roots(&self) -> Result<Vec<PathBuf>, UiStateError> {
        Ok(self
            .list_workspaces()?
            .iter()
            .map(|workspace| canonical_path(Path::new(&workspace.path)))
            .collect())
    }

    pub fn forget_workspace(&self, workspace_id: Uuid) -> Result<(), UiStateError> {
        self.with_database(|database| database.forget_workspace(workspace_id))
    }

    pub fn resolve_workspace(&self, path: &str, now_ms: i64) -> Result<Uuid, UiStateError> {
        self.with_database(|database| database.resolve_workspace(path, now_ms))
    }

    pub fn set_home_page(&self, path: &str, home_page: &str) -> Result<(), UiStateError> {
        self.with_database(|database| database.set_home_page(path, home_page))
    }

    pub fn resolve_document(
        &self,
        workspace_id: Uuid,
        relative_path: &str,
    ) -> Result<Uuid, UiStateError> {
        self.with_database(|database| database.resolve_document(workspace_id, relative_path))
    }

    pub fn open_session(&self, input: &OpenSessionInput) -> Result<LoadedSession, UiStateError> {
        validate_open_session(input)?;
        self.with_database(|database| database.open_session(input))
    }

    pub fn save_batch(&self, input: &SaveStateBatchInput) -> Result<bool, UiStateError> {
        validate_batch(input)?;
        self.with_database(|database| database.save_batch(input))
    }

    pub fn load_view(
        &self,
        workspace_id: Uuid,
        window_id: &str,
        document_id: Uuid,
        pane_id: &str,
    ) -> Result<Option<super::models::LoadedViewState>, UiStateError> {
        self.with_database(|database| {
            database.load_view(workspace_id, window_id, document_id, pane_id)
        })
    }

    pub fn rename_document(
        &self,
        document_id: Uuid,
        relative_path: &str,
    ) -> Result<(), UiStateError> {
        self.with_database(|database| database.rename_document(document_id, relative_path))
    }

    pub fn mark_document_missing(
        &self,
        document_id: Uuid,
        now_ms: i64,
    ) -> Result<(), UiStateError> {
        self.with_database(|database| database.mark_document_missing(document_id, now_ms))
    }

    pub fn cleanup(&self, retention_days: u32, now_ms: i64) -> Result<usize, UiStateError> {
        let cutoff_ms = now_ms.saturating_sub(i64::from(retention_days) * DAY_MS);
        self.with_database(|database| {
            let deleted = database.purge_missing(cutoff_ms, PURGE_BATCH)?;
            database.incremental_vacuum(VACUUM_PAGES)?;
            Ok(deleted)
        })
    }

    pub fn load_base_view(
        &self,
        workspace_id: Uuid,
        relative_path: &str,
        view_count: usize,
    ) -> Result<usize, UiStateError> {
        self.with_database(|database| {
            database.load_base_view(workspace_id, relative_path, view_count)
        })
    }

    pub fn save_base_view(
        &self,
        workspace_id: Uuid,
        relative_path: &str,
        selected_view: usize,
    ) -> Result<(), UiStateError> {
        self.with_database(|database| {
            database.save_base_view(workspace_id, relative_path, selected_view)
        })
    }

    pub fn load_reader_state(
        &self,
        workspace_id: Uuid,
        book_file: &str,
    ) -> Result<Option<super::models::LoadedReaderState>, UiStateError> {
        self.with_database(|database| database.load_reader_state(workspace_id, book_file))
    }

    pub fn save_reader_state(
        &self,
        input: &super::models::SaveReaderStateInput,
    ) -> Result<(), UiStateError> {
        self.with_database(|database| database.save_reader_state(input))
    }
}

fn open_storage(path: &Path) -> Storage {
    match UiStateDatabase::open(path) {
        Ok(database) => Storage::Ready(database),
        Err(error) => Storage::Unavailable(format!("{error:?}")),
    }
}

fn set_aside_database(path: &Path, now_ms: i64) -> Result<(), UiStateError> {
    for suffix in ["-wal", "-shm", ""] {
        let file = PathBuf::from(format!("{}{suffix}", path.display()));
        if file.exists() {
            std::fs::rename(&file, format!("{}.broken-{now_ms}", file.display()))?;
        }
    }
    Ok(())
}
