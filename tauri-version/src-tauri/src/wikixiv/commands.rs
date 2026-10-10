use crate::app_core::Core;
use std::sync::Arc;
use super::models::{SearchRequest, WikixivSearchResult};
use tauri::State;

#[tauri::command]
pub async fn wikixiv_search(
    core: State<'_, Arc<Core>>,
    text: String,
    generation: u64,
    document_path: Option<String>,
) -> Result<WikixivSearchResult, String> {
    let settings = &core.settings;
    let service = core.wikixiv.clone();
    let enabled = settings.get_config().analysis.enable_wikixiv;
    tauri::async_runtime::spawn_blocking(move || {
        service.search(
            SearchRequest {
                text,
                document_path,
                generation,
            },
            enabled,
        )
    })
    .await
    .map_err(|error| error.to_string())?
}
