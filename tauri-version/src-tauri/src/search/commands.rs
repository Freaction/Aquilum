use crate::app_core::Core;
use std::sync::Arc;
use super::dataview::QueryOutput;
use super::error::SearchError;
use crate::blocking::run_blocking;
use super::fields::NoteFields;
use super::models::{
    Backlink, NoteSuggestion, OutgoingLink, SearchIndexStatus, SearchResponse,
    WikiLinkResolution,
};
use tauri::State;

#[tauri::command]
pub async fn prepare_search_index(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
) -> Result<SearchIndexStatus, SearchError> {
    let core = Arc::clone(&core);
    run_blocking(move || {
        if let Ok(root) = super::paths::canonical_workspace(&workspace_path) {
            if let Err(error) = core.watcher.watch(&root) {
                eprintln!(
                    "[aquilum:watch] наблюдение за базой не запущено: {error}. \
                     Внешние правки будут подтягиваться при возврате фокуса в окно."
                );
            }
        }
        core.search.prepare(&workspace_path)
    })
    .await
}

#[tauri::command]
pub async fn search_knowledge_base(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    query: String,
    limit: Option<usize>,
) -> Result<SearchResponse, SearchError> {
    let service = core.search.clone();
    run_blocking(move || service.search(&workspace_path, &query, limit)).await
}

#[tauri::command]
pub fn get_search_index_status(
    core: State<'_, Arc<Core>>,
    workspace_path: Option<String>,
) -> SearchIndexStatus {
    let service = &core.search;
    service.status(workspace_path.as_deref())
}

#[tauri::command]
pub async fn get_backlinks(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    document_path: String,
) -> Result<Vec<Backlink>, SearchError> {
    let service = core.search.clone();
    run_blocking(move || service.backlinks(&workspace_path, &document_path)).await
}

#[tauri::command]
pub async fn get_outgoing_links(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    document_path: String,
) -> Result<Vec<OutgoingLink>, SearchError> {
    let service = core.search.clone();
    run_blocking(move || service.outgoing_links(&workspace_path, &document_path)).await
}

#[tauri::command]
pub async fn run_dataview_query(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    document_path: String,
    query: String,
    tz_offset_minutes: i64,
) -> Result<QueryOutput, SearchError> {
    let service = core.search.clone();
    run_blocking(move || {
        service.run_dataview_query(&workspace_path, &document_path, &query, tz_offset_minutes)
    })
    .await
}

#[tauri::command]
pub async fn get_note_fields(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    document_paths: Vec<String>,
) -> Result<Vec<NoteFields>, SearchError> {
    let service = core.search.clone();
    run_blocking(move || service.note_fields(&workspace_path, &document_paths)).await
}

#[tauri::command]
pub async fn resolve_wiki_links(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    source_path: String,
    targets: Vec<String>,
) -> Result<WikiLinkResolution, SearchError> {
    let service = core.search.clone();
    run_blocking(move || service.resolve_wiki_links(&workspace_path, &source_path, &targets)).await
}

#[tauri::command]
pub async fn suggest_notes(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<NoteSuggestion>, SearchError> {
    let service = core.search.clone();
    run_blocking(move || service.suggest_notes(&workspace_path, &query, limit.unwrap_or(12))).await
}
