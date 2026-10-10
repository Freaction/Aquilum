use crate::app_core::Core;
use std::sync::Arc;
use super::models::{AnalysisMethod, AnalysisResult};
use crate::blocking::run_blocking;
use crate::search::error::SearchError;
use tauri::State;


#[tauri::command]
pub async fn analyze_document(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    document_path: String,
    method: AnalysisMethod,
    limit: Option<usize>,
) -> Result<Vec<AnalysisResult>, SearchError> {
    let settings = &core.settings;
    let service = core.search.clone();
    let config = settings.get_config();
    run_blocking(move || {
        service.analyze_document(&workspace_path, &document_path, method, limit, &config)
    })
    .await
}
