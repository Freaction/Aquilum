use std::sync::atomic::{AtomicBool, Ordering};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;

static INSTALLING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
#[cfg_attr(not(feature = "updater"), allow(dead_code))]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum UpdateProgress {
    Downloading { chunk: usize, total: Option<u64> },
    Installing,
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<String>, String> {
    release::find(&app)
        .await
        .map(|update| update.map(|update| update.version))
}

#[tauri::command]
pub async fn install_update(app: AppHandle, on_progress: Channel<UpdateProgress>) -> Result<(), String> {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let result = match release::find(&app).await {
        Ok(Some(update)) => release::install(&app, update, on_progress).await,
        Ok(None) => Ok(()),
        Err(error) => Err(error),
    };
    INSTALLING.store(false, Ordering::SeqCst);
    result
}

#[cfg(feature = "updater")]
mod release {
    use super::UpdateProgress;
    use tauri::ipc::Channel;
    use tauri::AppHandle;
    use tauri_plugin_updater::{Update, UpdaterExt};
    use crate::app_core::Core;
    use std::sync::Arc;
    use tauri::Manager;

    pub async fn find(app: &AppHandle) -> Result<Option<Update>, String> {
        app.updater()
            .map_err(|error| error.to_string())?
            .check()
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn install(app: &AppHandle, update: Update, on_progress: Channel<UpdateProgress>) -> Result<(), String> {
        let installing = on_progress.clone();
        let core = Arc::clone(app.state::<Arc<Core>>().inner());
        update
            .download_and_install(
                move |chunk, total| {
                    let _ = on_progress.send(UpdateProgress::Downloading { chunk, total });
                },
                move || {
                    core.shutdown();
                    let _ = installing.send(UpdateProgress::Installing);
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        app.restart();
    }
}

#[cfg(not(feature = "updater"))]
mod release {
    use super::UpdateProgress;
    use tauri::ipc::Channel;
    use tauri::AppHandle;

    const UNAVAILABLE: &str = "unavailable";

    pub struct Update {
        pub version: String,
    }

    pub async fn find(_app: &AppHandle) -> Result<Option<Update>, String> {
        Err(UNAVAILABLE.to_string())
    }

    pub async fn install(_app: &AppHandle, _update: Update, _on_progress: Channel<UpdateProgress>) -> Result<(), String> {
        Err(UNAVAILABLE.to_string())
    }
}
