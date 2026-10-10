use super::service::{HistoryPage, VersionTexts};
use super::store;
use crate::blocking::run_blocking;
use crate::files::error::FileCommandError;
use crate::app_core::Core;
use crate::files::gate;
use crate::files::trash::trash_path;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

const HISTORY_PAGE_LIMIT: usize = 200;

#[tauri::command]
pub async fn note_history(
    core: State<'_, Arc<Core>>,
    path: String,
    offset: usize,
    limit: usize,
) -> Result<HistoryPage, FileCommandError> {
    let core = Arc::clone(&core);
    run_blocking(move || {
        let history = &core.history;
        Ok(history.page(&|| core.known_roots(), Path::new(&path), offset, limit.min(HISTORY_PAGE_LIMIT)))
    })
    .await
}

#[tauri::command]
pub async fn read_note_version(
    core: State<'_, Arc<Core>>,
    path: String,
    version: Option<String>,
) -> Result<Option<VersionTexts>, FileCommandError> {
    let core = Arc::clone(&core);
    run_blocking(move || {
        let history = &core.history;
        Ok(history.version(&|| core.known_roots(), Path::new(&path), version.as_deref()))
    })
    .await
}

#[tauri::command]
pub async fn name_note_version(
    core: State<'_, Arc<Core>>,
    path: String,
    version: Option<String>,
    name: String,
) -> Result<Option<String>, FileCommandError> {
    let core = Arc::clone(&core);
    run_blocking(move || Ok(gate::name_version(&core, Path::new(&path), version.as_deref(), &name))).await
}

#[tauri::command]
pub async fn cleanup_history(
    workspace_path: String,
    retention_days: u32,
) -> Result<u64, FileCommandError> {
    run_blocking(move || {
        let vault = Path::new(&workspace_path);
        store::adopt_quantum_history(vault);
        store::adopt_quantum_history(&trash_path(vault));
        Ok(store::remove_expired(vault, retention_days))
    })
    .await
}
