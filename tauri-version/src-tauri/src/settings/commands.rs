use crate::app_core::Core;
use std::sync::Arc;
use crate::settings::models::AppConfig;
use tauri::State;

#[tauri::command]
pub fn get_settings(core: State<'_, Arc<Core>>) -> Result<AppConfig, String> {
    let settings = &core.settings;
    Ok(settings.get_config())
}

#[tauri::command]
pub fn update_settings(
    core: State<'_, Arc<Core>>,
    new_config: AppConfig,
) -> Result<(), String> {
    let settings = &core.settings;
    settings.update_config(new_config)
}
