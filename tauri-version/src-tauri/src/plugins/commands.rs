use super::error::PluginError;
use super::vault_data::{self, VaultPluginData};
use crate::app_core::Core;
use crate::blocking::run_blocking;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn plugin_vault_data_get(
    _core: State<'_, Arc<Core>>,
    workspace_path: String,
) -> Result<VaultPluginData, PluginError> {
    run_blocking(move || vault_data::load(Path::new(&workspace_path))).await
}

#[tauri::command]
pub async fn plugin_vault_data_set(
    _core: State<'_, Arc<Core>>,
    workspace_path: String,
    data: VaultPluginData,
) -> Result<(), PluginError> {
    run_blocking(move || vault_data::save(Path::new(&workspace_path), &data)).await
}

#[tauri::command]
pub async fn calendar_month(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    year: i64,
    month: i64,
    locale: String,
    today: super::dates::LocalDateTime,
) -> Result<super::calendar::CalendarMonth, PluginError> {
    let core = core.inner().clone();
    run_blocking(move || {
        super::calendar::month(&core, Path::new(&workspace_path), year, month, &locale, today)
    }).await
}

#[tauri::command]
pub async fn calendar_open(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    period: String,
    date: String,
    create: bool,
    now: super::dates::LocalDateTime,
    locale: String,
) -> Result<Option<super::calendar::OpenedNote>, PluginError> {
    let core = core.inner().clone();
    run_blocking(move || {
        super::calendar::open(&core, Path::new(&workspace_path), &period, &date, create, now, &locale)
    }).await
}

// проводник.
#[tauri::command]
pub async fn explorer_overview(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
) -> Result<super::explorer::ExplorerOverview, PluginError> {
    let core = Arc::clone(core.inner());
    run_blocking(move || super::explorer::overview(&core, Path::new(&workspace_path))).await
}

// Git-синхронизация.
use super::git::{self, GitError, GitStatus};

#[tauri::command]
pub async fn git_status(workspace_path: String) -> Result<GitStatus, GitError> {
    run_blocking(move || git::status(Path::new(&workspace_path))).await
}

#[tauri::command]
pub async fn git_sync(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
    now: super::dates::LocalDateTime,
) -> Result<GitStatus, GitError> {
    let settings = core.settings.get_config().plugins.git_sync;
    run_blocking(move || {
        let message = git::commit_message(&settings.commit_message, &settings.commit_date_format, &now);
        git::sync(
            Path::new(&workspace_path), &message, &settings.sync_method, settings.push,
        )
    })
    .await
}

#[tauri::command]
pub async fn git_pull(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
) -> Result<GitStatus, GitError> {
    let method = core.settings.get_config().plugins.git_sync.sync_method;
    run_blocking(move || git::pull(Path::new(&workspace_path), &method)).await
}

// импорт настроек Obsidian.
use super::obsidian_import::{self, ImportReport};

#[tauri::command]
pub async fn obsidian_detect(workspace_path: String) -> Result<bool, PluginError> {
    run_blocking(move || Ok(obsidian_import::detect(Path::new(&workspace_path)))).await
}

#[tauri::command]
pub async fn obsidian_import(
    core: State<'_, Arc<Core>>,
    workspace_path: String,
) -> Result<ImportReport, PluginError> {
    let core = Arc::clone(core.inner());
    run_blocking(move || {
        let root = Path::new(&workspace_path);
        // ponytail: параллельный rename требует общего transaction API в vault_data.
        let original = vault_data::load(root)?;
        let result = obsidian_import::import(root, &core.settings.get_config(), &original)?;
        vault_data::save(root, &result.data)?;
        if let Err(message) = core.settings.update_config(result.config) {
            // Если настройки не сохранены, возвращаем прежние данные хранилища.
            vault_data::save(root, &original).map_err(|error| PluginError::Io {
                message: format!("{message}; не удалось восстановить данные плагинов: {error}"),
            })?;
            return Err(PluginError::Io { message });
        }
        Ok(result.report)
    }).await
}

#[tauri::command]
pub async fn table_format(markdown: String) -> Result<String, PluginError> {
    run_blocking(move || super::tables::format_table(&markdown)).await
}

#[tauri::command]
pub async fn table_sort(markdown: String, column: usize, descending: bool) -> Result<String, PluginError> {
    run_blocking(move || super::tables::sort_table(&markdown, column, descending)).await
}
