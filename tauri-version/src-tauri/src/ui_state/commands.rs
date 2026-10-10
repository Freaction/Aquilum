use crate::app_core::Core;
use std::sync::Arc;
use super::error::UiStateError;
use crate::blocking::run_blocking;
use super::models::{
    KnownWorkspace, LoadedReaderState, LoadedSession, OpenSessionInput, SaveReaderStateInput,
    SaveStateBatchInput,
};
use super::paths::{canonical_workspace, normalize_relative};
use crate::documents::DocumentHub;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn list_ui_workspaces(core: State<'_, Arc<Core>>) -> Result<Vec<KnownWorkspace>, UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || service.list_workspaces()).await
}

#[tauri::command]
pub async fn set_ui_workspace_home_page(
    core: State<'_, Arc<Core>>,
    path: String,
    home_page: String,
) -> Result<(), UiStateError> {
    let service = core.ui_state.clone();
    let canonical = canonical_workspace(&path)?;
    run_blocking(move || service.set_home_page(&canonical, &home_page)).await
}

#[tauri::command]
pub async fn forget_ui_workspace(
    core: State<'_, Arc<Core>>,
    workspace_id: Uuid,
) -> Result<(), UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || service.forget_workspace(workspace_id)).await
}

#[tauri::command]
pub async fn resolve_ui_workspace(
    core: State<'_, Arc<Core>>,
    path: String,
    now_ms: i64,
) -> Result<Uuid, UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || {
        let path = canonical_workspace(&path)?;
        service.resolve_workspace(&path, now_ms)
    })
    .await
}

#[tauri::command]
pub async fn resolve_ui_document(
    core: State<'_, Arc<Core>>,
    workspace_id: Uuid,
    relative_path: String,
) -> Result<Uuid, UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || {
        let relative_path = normalize_relative(&relative_path)?;
        service.resolve_document(workspace_id, &relative_path)
    })
    .await
}

#[tauri::command]
pub async fn open_ui_session(
    core: State<'_, Arc<Core>>,
    input: OpenSessionInput,
) -> Result<LoadedSession, UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || service.open_session(&input)).await
}

#[tauri::command]
pub async fn save_ui_state_batch(
    core: State<'_, Arc<Core>>,
    mut input: SaveStateBatchInput,
) -> Result<bool, UiStateError> {
    let core = Arc::clone(&core);
    run_blocking(move || {
        anchor_views(&core.documents, &mut input);
        core.ui_state.save_batch(&input)
    })
    .await
}

fn anchor_views(hub: &DocumentHub, input: &mut SaveStateBatchInput) {
    for view in &mut input.views {
        let Some(path) = view.path.as_deref() else { continue };
        let offsets = [view.fallback_anchor, view.fallback_head, view.fallback_scroll_anchor]
            .map(|offset| offset.max(0) as usize);
        let Some(encoded) = hub.encode_positions(Path::new(path), &offsets) else { continue };
        let [anchor, head, scroll] = <[Option<Vec<u8>>; 3]>::try_from(encoded).expect("three positions");
        view.cursor_anchor = anchor.unwrap_or_default();
        view.cursor_head = head.unwrap_or_default();
        view.scroll_anchor = scroll.unwrap_or_default();
    }
}

#[tauri::command]
pub async fn load_ui_document_view(
    core: State<'_, Arc<Core>>,
    workspace_id: Uuid,
    window_id: String,
    document_id: Uuid,
    pane_id: String,
) -> Result<Option<super::models::LoadedViewState>, UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || service.load_view(workspace_id, &window_id, document_id, &pane_id)).await
}

#[tauri::command]
pub async fn rename_ui_document(
    core: State<'_, Arc<Core>>,
    document_id: Uuid,
    relative_path: String,
) -> Result<(), UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || {
        let relative_path = normalize_relative(&relative_path)?;
        service.rename_document(document_id, &relative_path)
    })
    .await
}

#[tauri::command]
pub async fn mark_ui_document_missing(
    core: State<'_, Arc<Core>>,
    document_id: Uuid,
    now_ms: i64,
) -> Result<(), UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || service.mark_document_missing(document_id, now_ms)).await
}

#[tauri::command]
pub async fn cleanup_ui_state(
    core: State<'_, Arc<Core>>,
    retention_days: u32,
    now_ms: i64,
) -> Result<usize, UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || service.cleanup(retention_days, now_ms)).await
}

#[tauri::command]
pub async fn reset_ui_state(
    core: State<'_, Arc<Core>>,
    now_ms: i64,
) -> Result<(), UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || service.reset(now_ms)).await
}

#[tauri::command]
pub async fn load_ui_reader_state(
    core: State<'_, Arc<Core>>,
    workspace_id: Uuid,
    book_file: String,
) -> Result<Option<LoadedReaderState>, UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || {
        let book_file = normalize_relative(&book_file)?;
        service.load_reader_state(workspace_id, &book_file)
    })
    .await
}

#[tauri::command]
pub async fn save_ui_reader_state(
    core: State<'_, Arc<Core>>,
    input: SaveReaderStateInput,
) -> Result<(), UiStateError> {
    let service = core.ui_state.clone();
    run_blocking(move || {
        let book_file = normalize_relative(&input.book_file)?;
        service.save_reader_state(&SaveReaderStateInput {
            book_file,
            ..input
        })
    })
    .await
}
