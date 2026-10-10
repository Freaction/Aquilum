use tauri::WebviewWindow;

#[tauri::command]
pub fn pdf_export_is_native() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

#[tauri::command]
pub async fn export_pdf(window: WebviewWindow, path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        super::pdf::print_current_page(window, path).await
    }
    #[cfg(target_os = "macos")]
    {
        super::pdf_macos::print_current_page(window, path).await
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (window, path);
        Err("Печать в PDF из приложения доступна только в Windows и macOS".to_owned())
    }
}
