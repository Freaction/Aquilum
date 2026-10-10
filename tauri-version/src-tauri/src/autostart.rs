pub fn repoint_to_current_exe(app: &tauri::AppHandle) {
    use tauri_plugin_autostart::ManagerExt;

    let launcher = app.autolaunch();
    if launcher.is_enabled().unwrap_or(false) {
        if let Err(error) = launcher.enable() {
            eprintln!("[aquilum:autostart] не удалось обновить путь автозапуска: {error}");
        }
    }
}
