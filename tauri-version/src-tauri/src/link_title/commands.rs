#[tauri::command]
pub async fn fetch_page_title(url: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || super::fetch::page_title(&url))
        .await
        .map_err(|error| error.to_string())
}
