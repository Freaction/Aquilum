use super::McpStatus;
use crate::app_core::Core;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn set_active_note(core: State<'_, Arc<Core>>, path: Option<String>) {
    core.active_note.set(path.map(PathBuf::from));
}

#[tauri::command]
pub fn get_mcp_status(core: State<'_, Arc<Core>>) -> McpStatus {
    core.mcp.status()
}

#[tauri::command]
pub fn apply_mcp_settings(core: State<'_, Arc<Core>>) -> McpStatus {
    core.apply_mcp_settings()
}

#[tauri::command]
pub fn get_stdio_executable_path() -> String {
    // AppImage: $APPIMAGE contains the path to the running AppImage
    if let Ok(appimage) = env::var("APPIMAGE") {
        return appimage;
    }

    // Try current_exe first
    if let Ok(exe) = env::current_exe() {
        let exe_str = exe.to_string_lossy().to_string();

        // deb/rpm: standard Linux paths
        if cfg!(target_os = "linux") {
            if exe_str.starts_with("/usr/bin/") || exe_str.starts_with("/usr/local/bin/") {
                return exe_str;
            }
            // Flatpak: /var/lib/flatpak/exports/bin/...
            if exe_str.starts_with("/var/lib/flatpak/") {
                return exe_str;
            }
        }

        // macOS: .app bundle
        if cfg!(target_os = "macos") {
            if exe_str.ends_with("MacOS/aquilum-app") || exe_str.contains(".app/Contents/MacOS") {
                return exe_str;
            }
        }

        // Windows
        if cfg!(target_os = "windows") {
            return exe_str;
        }

        return exe_str;
    }

    String::new()
}
