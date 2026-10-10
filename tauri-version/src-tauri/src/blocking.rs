use crate::app_core::TaskFailed;

pub(crate) async fn run_blocking<T, E, F>(operation: F) -> Result<T, E>
where
    T: Send + 'static,
    E: From<TaskFailed> + Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| E::from(TaskFailed(error.to_string())))?
}
